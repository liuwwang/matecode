pub mod archive;
pub mod branch;
pub mod commit;
pub mod init;
pub mod install_hook;
pub mod linter;
pub mod report;
pub mod review;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// 初始化配置文件
    #[command(alias = "i")]
    Init,

    /// 记录每个项目的git信息
    #[command(hide = true)]
    Archive,

    /// 安装git钩子，搭配archive使用完成自动归档
    InstallHook,

    /// AI生成暂存空间内的git commit 信息并commit
    #[command(alias = "c")]
    Commit {
        /// 自动暂存所有已跟踪的文件修改，等同于`git add -u`操作
        #[arg(short, long)]
        all: bool,

        /// 提交前运行lint
        #[arg(long)]
        lint: bool,

        /// 启用结构化提交模式，以交互方式添加元数据
        #[arg(short, long)]
        structured: bool,
    },

    /// AI生成工作报告,支持指定起始日期或预定义周期
    #[command(alias = "r")]
    Report {
        /// 开始时间
        #[arg(short, long)]
        since: Option<String>,

        /// 结束时间
        #[arg(short, long)]
        until: Option<String>,

        /// 预定义时间周期: today/t(今天), week/w(最近一周), month/m(最近一个月), quarter/q(最近一个季度), year/y(最近一年)
        #[arg(short, long)]
        period: Option<String>,
    },

    /// 进行代码风格检查
    Lint {
        /// 显示详情
        #[arg(short, long)]
        detail: bool,
    },

    /// 辅助你完成review代码
    #[command(alias = "rev")]
    Review {
        /// 启用lint
        #[arg(short, long)]
        lint: bool,
    },

    /// AI生成分支名称
    #[command(alias = "b")]
    Branch {
        /// 功能描述
        description: String,

        /// 直接创建并切换到新分支
        #[arg(short, long)]
        create: bool,

        /// 基于当前暂存区变更生成分支名
        #[arg(long)]
        from_staged: bool,
    },
}
