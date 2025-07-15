//! src/main.rs

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Select, Confirm};
use std::path::Path;

mod cli;
mod config;
mod git;
mod history;
mod hook;
mod llm;

use cli::{Cli, Commands};
use git::{get_project_name, get_staged_diff, run_git_command};
use llm::generate_commit_message;

async fn run() -> Result<()> {
    // 跨平台的环境变量加载
    // 1. 首先尝试从配置目录加载 .env 文件
    if let Ok(config_dir) = config::get_config_dir() {
        let env_path = config_dir.join(".env");
        if env_path.exists() {
            dotenvy::from_path(env_path).ok();
        }
    }

    // 2. 也尝试从当前工作目录加载 .env 文件
    if Path::new(".env").exists() {
        dotenvy::dotenv().ok();
    }

    let cli = Cli::parse();

    match cli.command {
        Commands::Commit { all } => {
            if all {
                run_git_command(&["add", "-u"])
                    .context("Failed to stage all tracked files.")?;
                println!("{}", "Staged all tracked files.".green());
            }
            
            loop {
                let diff = git::get_staged_diff()
                    .context("Failed to get staged git diff.")?;

                if diff.is_empty() {
                    println!("{}", "No staged changes found.".yellow());
                    return Ok(());
                }
    
                let llm_client = config::get_llm_client()?;
                let mut commit_message = generate_commit_message(&llm_client, &diff).await?;
                commit_message = commit_message.replace('`', "'");
    
                println!("\n{}\n", "=".repeat(60));
                println!("{}", commit_message.cyan());
                println!("{}\n", "=".repeat(60));

                let options = &[
                    "✅ 直接提交 (Apply)",
                    "📝 编辑后提交 (Edit)",
                    "🔄 重新生成 (Regenerate)",
                    "❌ 退出 (Quit)",
                ];

                let selection = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("您想如何处理这条提交信息？")
                    .items(&options[..])
                    .default(0)
                    .interact()?;

                match selection {
                    0 => {
                        // 直接提交
                        let lines: Vec<&str> = commit_message.lines().collect();
                        let mut cmd_args: Vec<&str> = vec!["commit"];
                        for line in &lines {
                            cmd_args.push("-m");
                            cmd_args.push(line);
                        }
                        run_git_command(&cmd_args)
                            .context("Failed to execute git commit.")?;
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
                            let lines: Vec<&str> = edited_message.lines().collect();
                            let mut cmd_args: Vec<&str> = vec!["commit"];
                            for line in &lines {
                                cmd_args.push("-m");
                                cmd_args.push(line);
                            }
                            run_git_command(&cmd_args)
                                .context("Failed to execute git commit after editing.")?;
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
        },
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
            
            let all_commits = history::get_all_commits_in_range(start_date, end_date).await
                .context("Failed to get commit history for the report.")?;

            if all_commits.is_empty() {
                println!("{}", "在此日期范围内没有找到任何提交记录。".yellow());
                return Ok(());
            }

            let llm_client = config::get_llm_client()?;
            let report = llm::generate_report_from_commits(&llm_client, &all_commits, start_date, end_date).await?;
            println!("{report}");
        }
        Commands::Review => {
            let diff = get_staged_diff()
                .context("Failed to get staged git diff for review.")?;

            if diff.is_empty() {
                println!("{}", "没有需要审查的暂存更改。".yellow());
                return Ok(());
            }

            println!("🤖 正在审查您的代码，请稍候...");

            let llm_client = config::get_llm_client()?;
            let review = llm::generate_code_review(&llm_client, &diff).await?;
            
            println!("\n{}\n", "=".repeat(60));
            println!("📝 AI 代码审查报告:");
            println!("{}\n", "=".repeat(60));
            println!("{}", review);
        }
        Commands::Init => {
            let config_path = config::create_default_config()
                .await
                .context("Failed to initialize configuration.")?;
            println!(
                "{}{}{}",
                "配置文件初始化成功，位于 ".green(),
                config_path.to_str().unwrap().green(),
                "/".green()
            );
        }
        Commands::Archive => {
            let project_name = git::get_project_name()
                .context("Failed to get project name for archiving.")?;
            let commit_message = git::get_last_commit_message()
                .context("Failed to get last commit message for archiving.")?;
            history::archive_commit_message(&project_name, &commit_message).await
                .context("Failed to archive commit message.")?;
            // 注意：此处不再直接归档
        }
        Commands::InstallHook => {
            if let Err(e) = hook::install_post_commit_hook() {
                eprintln!("{} {}", "钩子安装失败:".red(), e.to_string().red());
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{}: {:?}", "错误".red(), e);
        std::process::exit(1);
    }
}
