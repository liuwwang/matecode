//! src/cli.rs
use clap::{Parser, Subcommand};

/// 一个用来自动生成 Git Commit 和工作日报的 CLI 工具
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// 生成并执行 git commit 命令
    Commit {
        /// 自动暂存所有已跟踪的文件的更改
        #[clap(short, long)]
        all: bool,

        /// 在提交前运行 linter
        #[clap(long)]
        lint: bool,
    },
    /// 根据提交历史生成工作日报
    Report {
        /// 报告的开始日期 (例如, "yesterday", "2 days ago", "2023-01-01")
        #[arg(long)]
        since: Option<String>,

        /// 报告的结束日期 (例如 "2023-01-31")。默认为今天
        #[arg(short, long)]
        until: Option<String>,
    },
    /// 对暂存的更改进行 AI 代码审查
    Review {
        /// 在审查前运行 linter
        #[clap(long)]
        lint: bool,
    },
    /// 对代码库运行 linter
    Lint {
        /// 显示 linter 输出的详细信息
        #[arg(long)]
        details: bool,
    },
    /// 初始化 matecode 配置文件
    #[command(alias = "i")]
    Init,
    /// [内部] 归档最后一条提交信息，由 git hooks 使用
    #[command(hide = true)]
    Archive,

    /// 安装 git hook
    InstallHook,
}
