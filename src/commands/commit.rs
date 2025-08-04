use crate::commands::install_hook::{check_hook_status, install_post_commit_hook, HookStatus};
use crate::commands::linter::{handle_linter, parse_linter_summary};
use crate::config;
use crate::git;
use crate::llm::generate_commit_message;
use anyhow;
use anyhow::Context;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};

async fn prompt_for_metadata() -> anyhow::Result<String> {
    let mut footer = String::new();

    let issue: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("关联的 Issue ID 是什么？(选填, e.g., PROJ-123)")
        .allow_empty(true)
        .interact_text()?;

    if !issue.trim().is_empty() {
        footer.push_str(&format!("\nIssue: {}", issue.trim()));
    }

    let risk_levels = &["low", "medium", "high"];
    let risk_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("这次变更的风险等级是？")
        .items(risk_levels)
        .default(0)
        .interact()?;

    footer.push_str(&format!("\nRisk-Level: {}", risk_levels[risk_selection]));

    if Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("这是否是一个破坏性变更 (Breaking Change)？")
        .default(false)
        .interact()?
    {
        let breaking_change_description: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("请简要描述这个破坏性变更:")
            .allow_empty(false)
            .interact_text()?;
        footer.push_str(&format!(
            "\n\nBREAKING CHANGE: {}",
            breaking_change_description
        ));
    }

    Ok(footer)
}

pub async fn handle_commit(all: bool, lint: bool, structured: bool) -> anyhow::Result<()> {
    if !git::check_is_git_repo().await {
        eprintln!("{}", "错误: 当前目录不是一个有效的 Git 仓库。".red());
        return Ok(());
    }

    if lint {
        println!("{}", "(--lint) 提交前运行linter...".bold());
        let lint_result = handle_linter(false).await?;
        if let Some(output) = lint_result {
            if parse_linter_summary(&output).is_some() {
                if !Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("Lint 检查发现问题。确定还要提交吗")
                    .default(false)
                    .interact()?
                {
                    println!("提交已取消.");
                    return Ok(());
                }
            }
        }
        println!("{}", "-".repeat(60));
    }

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
        HookStatus::InstalledByUs => {}
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

    let diff = git::get_staged_diff()
        .await
        .context("无法获取暂存的git diff")?;

    if diff.is_empty() {
        println!("{}", "没有发现暂存的修改.".green());
        return Ok(());
    }

    let llm_client = config::get_llm_client().await?;
    let mut commit_message = generate_commit_message(llm_client.as_client(), &diff).await?;
    commit_message = commit_message.replace('`', "'");

    loop {
        println!("\n{}\n", "=".repeat(60));
        println!("{}", commit_message.cyan());
        println!("{}\n", "=".repeat(60));

        let options = &["✅ 直接提交", "🔄 重新生成", "💬 AI对话改进", "❌ 退出"];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("您想如何处理这条提交信息？")
            .items(&options[..])
            .default(0)
            .interact()?;

        match selection {
            0 => {
                let mut final_commit_message = commit_message;
                if structured {
                    let metadata_footer = prompt_for_metadata().await?;
                    if !metadata_footer.is_empty() {
                        final_commit_message.push_str("\n");
                        final_commit_message.push_str(&metadata_footer);
                    }
                }
                git::run_git_command(&["commit", "-m", &final_commit_message])
                    .await
                    .context("无法执行 git commit。")?;
                println!("🚀 提交成功！");
                break;
            }
            1 => {
                println!("🔄 好的，正在为您重新生成...");
                commit_message = generate_commit_message(llm_client.as_client(), &diff).await?;
                commit_message = commit_message.replace('`', "'");
                continue;
            }
            2 => {
                let mut message_for_improvement = commit_message.clone();
                loop {
                    let user_feedback: String = Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("💬 请告诉我您希望如何改进这条提交信息")
                        .allow_empty(false)
                        .interact_text()?;

                    if user_feedback.trim().is_empty() {
                        println!("未输入任何反馈，返回选择菜单。");
                        break;
                    }

                    println!("🤖 正在根据您的反馈改进提交信息...");
                    let improvement_prompt = format!(
                        "用户对以下提交信息有改进建议：\n\n当前提交信息：\n{}\n\n用户反馈：\n{}\n\n代码变更内容：\n{}\n\n请根据用户的反馈和代码变更内容改进提交信息，保持简洁明了，符合conventional commits格式。只返回改进后的提交信息，不要添加额外的解释。",
                        message_for_improvement, user_feedback, diff
                    );

                    match llm_client
                        .as_client()
                        .call(
                            "你是一个专业的Git提交信息助手，擅长根据用户反馈改进提交信息。",
                            &improvement_prompt,
                        )
                        .await
                    {
                        Ok(improved_message) => {
                            let final_improved_message =
                                improved_message.replace('`', "'").trim().to_string();

                            println!("\n{}", "=".repeat(60));
                            println!("{}", "改进后的提交信息:".green());
                            println!("{}", final_improved_message.cyan());
                            println!("{}", "=".repeat(60));

                            let feedback_options =
                                &["✅ 使用改进后的版本", "🔄 继续改进", "↩️ 放弃本次改进"];
                            let feedback_selection =
                                Select::with_theme(&ColorfulTheme::default())
                                    .with_prompt("您对改进后的提交信息满意吗？")
                                    .items(&feedback_options[..])
                                    .default(0)
                                    .interact()?;

                            match feedback_selection {
                                0 => {
                                    commit_message = final_improved_message;
                                    println!("✨ 已采用改进后的提交信息，返回主菜单。");
                                    break;
                                }
                                1 => {
                                    message_for_improvement = final_improved_message;
                                    println!("🔄 好的，请继续告诉我您的改进建议：");
                                    continue;
                                }
                                2 => {
                                    println!("↩️ 已放弃本次改进，返回主菜单。");
                                    break;
                                }
                                _ => unreachable!(),
                            }
                        }
                        Err(e) => {
                            println!("❌ 改进提交信息时出错: {}", e);
                            if !Confirm::with_theme(&ColorfulTheme::default())
                                .with_prompt("是否重试？")
                                .default(false)
                                .interact()?
                            {
                                break;
                            }
                        }
                    }
                }
                continue;
            }
            3 => {
                println!("好的，操作已取消。");
                break;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}
