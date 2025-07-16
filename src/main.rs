//! src/main.rs

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Select};

mod cli;
mod config;
mod git;
mod history;
mod hook;
mod llm;

use cli::{Cli, Commands};
use git::get_staged_diff;
use hook::{check_hook_status, install_post_commit_hook, HookStatus};
use llm::generate_commit_message;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Commit { all } => {
            if !git::is_git_repository().await {
                eprintln!("{}", "错误: 当前目录不是一个有效的 Git 仓库。".red());
                return Ok(());
            }

            // 智能安装 Git 钩子
            match check_hook_status().await? {
                HookStatus::NotInstalled => {
                    if Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("检测到您尚未安装 matecode 的 post-commit 钩子，它能帮助自动记录提交历史以生成报告。是否立即为您安装？")
                        .default(true)
                        .interact()?
                    {
                        install_post_commit_hook().await?;
                    } else {
                        println!("好的，已跳过安装。您可以随时手动运行 `matecode install-hook`。");
                    }
                }
                HookStatus::InstalledByOther => {
                    if Confirm::with_theme(&ColorfulTheme::default())
                        .with_prompt("检测到已存在一个自定义的 post-commit 钩子。是否要将 `matecode archive` 命令添加到现有钩子中？")
                        .default(true)
                        .interact()?
                    {
                        install_post_commit_hook().await?;
                    } else {
                        println!("{}", "警告: 为确保 matecode 的报告功能正常工作，请将 `matecode archive` 命令手动添加到您现有的钩子脚本中。".yellow());
                    }
                }
                HookStatus::InstalledByUs => {
                    // 已安装，无需任何操作
                }
            }


            if all {
                git::run_git_command(&["add", "-u"])
                    .await
                    .context("无法暂存所有已跟踪的文件。")?;
                
                let staged_files = git::get_staged_files().await?;
                if staged_files.is_empty() {
                    println!("{}", "没有可暂存的已跟踪文件。".yellow());
                } else {
                    println!("{}", "已暂存以下文件的变更:".green());
                    for file in staged_files {
                        println!("  - {}", file.cyan());
                    }
                }
            }

            loop {
                let diff =
                    get_staged_diff().await.context("无法获取暂存的 git diff。")?;

                if diff.is_empty() {
                    println!("{}", "没有发现暂存的修改。".yellow());
                    return Ok(());
                }

                let llm_client = config::get_llm_client().await?;
                let mut commit_message =
                    generate_commit_message(llm_client.as_client(), &diff).await?;
                commit_message = commit_message.replace('`', "'");

                println!("\n{}\n", "=".repeat(60));
                println!("{}", commit_message.cyan());
                println!("{}\n", "=".repeat(60));

                let options = &[
                    "✅ 直接提交",
                    "📝 编辑后提交",
                    "🔄 重新生成",
                    "❌ 退出",
                ];

                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("您想如何处理这条提交信息？")
                    .items(&options[..])
                    .default(0)
                    .interact()?;

                match selection {
                    0 => {
                        // 直接提交
                        git::run_git_command(&["commit", "-m", &commit_message])
                            .await
                            .context("无法执行 git commit。")?;
                        println!("🚀 提交成功！");
                        break;
                    }
                    1 => {
                        // 编辑后提交
                        let edited_message = edit::edit(&commit_message)?;

                        if edited_message.trim().is_empty() {
                            println!("编辑后的消息为空，提交已中止。");
                            break;
                        }

                        println!("\n📝 这是您编辑后的提交信息:\n");
                        println!("{}\n", "=".repeat(60));
                        println!("{}", edited_message.cyan());
                        println!("{}\n", "=".repeat(60));

                        if Confirm::with_theme(&ColorfulTheme::default())
                            .with_prompt("确认要提交吗?")
                            .default(true)
                            .interact()?
                        {
                            git::run_git_command(&["commit", "-m", &edited_message])
                                .await
                                .context("编辑后无法执行 git commit。")?;
                            println!("🚀 提交成功！");
                        } else {
                            println!("好的，提交已取消。");
                        }
                        break;
                    }
                    2 => {
                        // 重新生成
                        println!("🔄 好的，正在为您重新生成...");
                        continue;
                    }
                    3 => {
                        // 退出
                        println!("好的，操作已取消。");
                        break;
                    }
                    _ => unreachable!(),
                }
            }
        }
        Commands::Report { since, until } => {
            let now = chrono::Local::now().date_naive();

            let start_date = since
                .and_then(|s| dateparser::parse(&s).ok())
                .map(|dt| dt.date_naive())
                .unwrap_or(now);

            let end_date = until
                .and_then(|s| dateparser::parse(&s).ok())
                .map(|dt| dt.date_naive())
                .unwrap_or(now);

            let all_commits =
                history::get_all_commits_in_range(start_date, end_date)
                    .await
                    .context("无法获取用于报告的提交历史。")?;

            if all_commits.is_empty() {
                println!("{}", "在此日期范围内没有找到任何提交记录。".yellow());
                return Ok(());
            }

            let llm_client = config::get_llm_client().await?;
            let report = llm::generate_report_from_commits(
                llm_client.as_client(),
                &all_commits,
                start_date,
                end_date,
            )
            .await?;
            println!("{report}");
        }
        Commands::Review => {
            let diff = get_staged_diff()
                .await
                .context("无法获取用于审查的暂存 git diff。")?;

            if diff.is_empty() {
                println!("{}", "没有需要审查的暂存更改。".yellow());
                return Ok(());
            }

            let llm_client = config::get_llm_client().await?;
            let review = llm::generate_code_review(llm_client.as_client(), &diff).await?;

            println!("\n{}\n", "=".repeat(60));
            println!("📝 AI 代码审查报告:");
            println!("{}\n", "=".repeat(60));
            println!("{}", review);
        }
        Commands::Init => {
            config::create_default_config()
                .await
                .context("无法初始化配置。")?;
        }
        Commands::Archive => {
            let project_name = git::get_project_name()
                .await
                .context("无法获取用于归档的项目名称。")?;
            let commit_message = git::get_last_commit_message()
                .await
                .context("无法获取用于归档的最后一条提交信息。")?;
            history::archive_commit_message(&project_name, &commit_message)
                .await
                .context("无法归档提交信息。")?;
        }
        Commands::InstallHook => {
            hook::install_post_commit_hook().await?;
        }
    }

    Ok(())
}
