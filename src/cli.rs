//! src/cli.rs
use clap::{Parser, Subcommand};

/// 一个用来自动生成 Git Commit 和工作日报的 CLI 工具
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Commits the staged changes with a generated message.
    Commit {
        /// Optional: Add a scope to the commit message.
        #[clap(short, long)]
        scope: Option<String>,
    },
    /// Generates a report of the work done today.
    Report {
        /// Optional: Specify the author to generate the report for.
        #[clap(short, long)]
        author: Option<String>,
    },
    /// Initializes matecode's configuration.
    Init,
}
