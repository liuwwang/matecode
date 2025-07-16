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
    /// 根据暂存的变更生成提交信息
    #[command(alias = "c")]
    Commit {
        /// 在提交前暂存所有修改和删除的文件，类似于 `git commit -a`
        #[arg(short, long)]
        all: bool,
    },
    /// 根据提交历史生成工作报告
    #[command(alias = "r")]
    Report {
        /// 报告的开始日期 (例如 "2023-01-01", "7d ago")。默认为今天
        #[arg(long)]
        since: Option<String>,

        /// 报告的结束日期 (例如 "2023-01-31")。默认为今天
        #[arg(long)]
        until: Option<String>,
    },
    /// 对暂存的代码变更进行 AI 审查
    #[command(alias = "rev")]
    Review,
    /// 初始化 matecode 配置文件
    #[command(alias = "i")]
    Init,
    /// [内部] 归档最后一条提交信息，由 git hooks 使用
    #[command(hide = true)]
    Archive,

    /// 安装 git hook
    InstallHook,
}
