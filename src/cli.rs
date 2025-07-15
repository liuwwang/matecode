//! src/cli.rs
use clap::{Parser, Subcommand};

/// 一个用来自动生成 Git Commit 和工作日报的 CLI 工具
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate a commit message based on staged changes.
    #[command(alias = "c")]
    Commit {
        /// Stage all modified and deleted files before committing, same as `git commit -a`
        #[arg(short, long)]
        all: bool,
    },
    /// Generate a work report based on commit history.
    #[command(alias = "r")]
    Report {
        /// The start date for the report (e.g., "2023-01-01", "7d ago"). Defaults to today.
        #[arg(long)]
        since: Option<String>,

        /// The end date for the report (e.g., "2023-01-31"). Defaults to today.
        #[arg(long)]
        until: Option<String>,
    },
    /// Perform an AI-powered review of staged code changes.
    #[command(alias = "rev")]
    Review,
    /// Initialize matecode configuration file.
    #[command(alias = "i")]
    Init,
    /// [Internal] Archive the last commit message, used by git hooks.
    #[command(hide = true)]
    Archive,

    /// 安装 git hook
    InstallHook,
}
