//! src/cli.rs
use clap::{Parser, Subcommand};

/// 一个用来自动生成 Git Commit 和工作日报的 CLI 工具
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// 生成并打印提交信息
    Commit,
    /// 生成并打印日报
    Report,
    /// 初始化 matecode 配置文件
    Init,
    /// [内部使用] 归档上一次的提交信息，用于 git hook
    Archive,

    /// 安装 git hook
    InstallHook,
}
