pub mod archive;
pub mod commit;
pub mod init;
pub mod install_hook;
pub mod report;
pub mod understand;

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

        /// 启用结构化提交模式，以交互方式添加元数据
        #[arg(short, long)]
        structured: bool,

        /// [测试用] 禁用交互式编辑
        #[arg(long, hide = true)]
        no_edit: bool,
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

    /// AI理解项目结构和功能
    Understand {
        /// 指定要分析的目录路径，默认为当前git仓库根目录
        #[arg(short, long)]
        dir: Option<String>,
    },
}
