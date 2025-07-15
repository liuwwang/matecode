//! src/main.rs

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Select};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;

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
                run_git_command(&["add", "-u"])?;
                println!("{}", "Staged all tracked files.".green());
            }

            loop {
                let diff = get_staged_diff()?;

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
                        run_git_command(&cmd_args)?;
                        println!("🚀 提交成功！");
                        break;
                    }
                    1 => {
                        // 编辑后提交
                        let git_dir =
                            String::from_utf8(run_git_command(&["rev-parse", "--git-dir"])?.stdout)?
                                .trim()
                                .to_string();
                        let commit_editmsg_path = Path::new(&git_dir).join("COMMIT_EDITMSG");
                        let mut file = File::create(&commit_editmsg_path)?;
                        file.write_all(commit_message.as_bytes())?;
                        
                        let status = Command::new("git").arg("commit").arg("-e").status()?;

                        if status.success() {
                            println!("🚀 提交成功！");
                        } else {
                            println!("提交已中止。");
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
        Commands::Report { .. } => {
            let llm_client = config::get_llm_client()?;
            let report = llm::generate_daily_report(&llm_client).await?;
            println!("{report}");
        }
        Commands::Init => {
            let config_path = config::create_default_config()
                .await
                .expect("Failed to create default config");
            println!(
                "{}{}{}",
                "配置文件初始化成功，位于 ".green(),
                config_path.to_str().unwrap().green(),
                "/".green()
            );
        }
        Commands::Archive => {
            let project_name = git::get_project_name()?;
            let commit_message = git::get_last_commit_message()?;
            history::archive_commit_message(&project_name, &commit_message)?;
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
