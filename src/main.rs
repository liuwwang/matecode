//! src/main.rs

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use dotenvy;
use indicatif::{ProgressBar, ProgressStyle};
use matecode::{
    cli::{Cli, Commands},
    config, git, llm,
};
use std::path::Path;
use std::time::Duration;

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
        Commands::Commit { .. } => {
            let diff = git::get_staged_diff()?;

            if diff.is_empty() {
                println!("{}", "未发现暂存的变更。".yellow());
                return Ok(());
            }

            let client = config::get_llm_client()?;

            let spinner = ProgressBar::new_spinner();
            spinner.set_style(
                ProgressStyle::default_spinner()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                    .template("{spinner:.blue} {msg}")?,
            );
            spinner.set_message("正在生成提交信息...");
            spinner.enable_steady_tick(Duration::from_millis(100));

            let message = llm::generate_commit_message(&client, &diff).await?;

            spinner.finish_and_clear();

            println!("{}", message);
        }
        Commands::Report { .. } => {
            println!("{}", "Report 命令暂未实现。".yellow());
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
