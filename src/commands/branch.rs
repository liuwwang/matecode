use crate::config;
use crate::config::get_prompt_template;
use crate::git;
use crate::llm::{LLMClient, parse_prompt_template};
use anyhow::{Context, Result, anyhow};
use colored::Colorize;

/// 构建分支生成的用户提示词
fn build_branch_user_prompt(template: &str, description: &str, staged_context: &str) -> String {
    template
        .replace("{description}", description)
        .replace("{staged_context}", staged_context)
}

/// 从 LLM 响应中提取分支名称
fn extract_branch_name(response: &str) -> Option<String> {
    let start_tag = "<branch_name>";
    let end_tag = "</branch_name>";

    let start = response.find(start_tag)? + start_tag.len();
    let end = response.find(end_tag)?;

    Some(response[start..end].trim().to_string())
}

/// 生成分支名称
async fn generate_branch_name(client: &dyn LLMClient, description: &str, staged_context: &str) -> Result<String> {
    let template = get_prompt_template("branch").await?;
    let (system_prompt, user_prompt) = parse_prompt_template(&template)?;

    let user_prompt = build_branch_user_prompt(&user_prompt, description, staged_context);

    let response = client.call(&system_prompt, &user_prompt).await?;

    extract_branch_name(&response)
        .ok_or_else(|| anyhow!("无法从 LLM 响应中提取有效的分支名称"))
}

/// 获取暂存区上下文信息
async fn get_staged_context() -> Result<String> {
    let staged_files = git::get_staged_files().await?;

    if staged_files.is_empty() {
        return Ok(String::new());
    }

    let staged_diff = git::get_staged_diff().await?;
    let context = format!(
        "当前暂存区信息:\n文件: {}\n\n变更概要:\n{}",
        staged_files.join(", "),
        if staged_diff.len() > 500 {
            format!("{}...(已截断)", &staged_diff[..500])
        } else {
            staged_diff
        }
    );

    Ok(context)
}

/// 智能生成分支名称（不依赖 LLM）
pub fn generate_smart_branch_name(description: &str) -> String {
    // 中文到英文的简单映射
    let translated = translate_to_english(description);

    // 清理和格式化分支名称
    let sanitized = translated
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-')
        .collect::<String>()
        .split_whitespace()
        .take(4)  // 增加到4个词以获得更好的描述性
        .collect::<Vec<_>>()
        .join("-");

    // 根据描述内容判断分支类型
    let prefix = determine_branch_type(&translated);

    format!("{}/{}", prefix, sanitized)
}

/// 简单的中文到英文翻译映射
fn translate_to_english(description: &str) -> String {
    let mut result = description.to_string();

    // 常见的中文词汇映射
    let translations = [
        ("修复", "fix"),
        ("修改", "fix"),
        ("解决", "fix"),
        ("bug", "bug"),
        ("错误", "bug"),
        ("问题", "issue"),
        ("添加", "add"),
        ("新增", "add"),
        ("增加", "add"),
        ("创建", "create"),
        ("实现", "implement"),
        ("开发", "develop"),
        ("功能", "feature"),
        ("特性", "feature"),
        ("重构", "refactor"),
        ("优化", "optimize"),
        ("改进", "improve"),
        ("更新", "update"),
        ("升级", "upgrade"),
        ("删除", "remove"),
        ("移除", "remove"),
        ("文档", "docs"),
        ("说明", "docs"),
        ("测试", "test"),
        ("单元测试", "unit-test"),
        ("集成测试", "integration-test"),
        ("性能", "performance"),
        ("配置", "config"),
        ("设置", "config"),
        ("用户", "user"),
        ("登录", "login"),
        ("注册", "register"),
        ("认证", "auth"),
        ("权限", "permission"),
        ("数据库", "database"),
        ("接口", "api"),
        ("界面", "ui"),
        ("页面", "page"),
        ("组件", "component"),
        ("模块", "module"),
        ("服务", "service"),
        ("工具", "tool"),
        ("脚本", "script"),
        ("命令", "command"),
        ("参数", "param"),
        ("变量", "variable"),
        ("方法", "method"),
        ("函数", "function"),
        ("类", "class"),
        ("结构", "structure"),
        ("架构", "architecture"),
        ("框架", "framework"),
        ("库", "library"),
        ("依赖", "dependency"),
        ("包", "package"),
        ("版本", "version"),
        ("发布", "release"),
        ("部署", "deploy"),
        ("构建", "build"),
        ("编译", "compile"),
        ("打包", "package"),
        ("安装", "install"),
        ("卸载", "uninstall"),
        ("启动", "start"),
        ("停止", "stop"),
        ("重启", "restart"),
        ("运行", "run"),
        ("执行", "execute"),
        ("处理", "handle"),
        ("管理", "manage"),
        ("控制", "control"),
        ("监控", "monitor"),
        ("日志", "log"),
        ("记录", "record"),
        ("报告", "report"),
        ("统计", "statistics"),
        ("分析", "analysis"),
        ("搜索", "search"),
        ("查询", "query"),
        ("过滤", "filter"),
        ("排序", "sort"),
        ("分页", "pagination"),
        ("缓存", "cache"),
        ("存储", "storage"),
        ("备份", "backup"),
        ("恢复", "restore"),
        ("同步", "sync"),
        ("异步", "async"),
        ("并发", "concurrent"),
        ("线程", "thread"),
        ("进程", "process"),
        ("队列", "queue"),
        ("消息", "message"),
        ("通知", "notification"),
        ("邮件", "email"),
        ("短信", "sms"),
        ("支付", "payment"),
        ("订单", "order"),
        ("商品", "product"),
        ("购物车", "cart"),
        ("地址", "address"),
        ("位置", "location"),
        ("地图", "map"),
        ("图片", "image"),
        ("文件", "file"),
        ("上传", "upload"),
        ("下载", "download"),
        ("导入", "import"),
        ("导出", "export"),
        ("格式", "format"),
        ("解析", "parse"),
        ("验证", "validate"),
        ("校验", "validate"),
        ("检查", "check"),
        ("扫描", "scan"),
        ("清理", "clean"),
        ("整理", "organize"),
    ];

    // 应用翻译映射，在替换时添加空格分隔
    for (chinese, english) in &translations {
        if result.contains(chinese) {
            result = result.replace(chinese, &format!(" {} ", english));
        }
    }

    // 清理多余的空格
    result = result.split_whitespace().collect::<Vec<_>>().join(" ");

    result
}

/// 根据描述确定分支类型
fn determine_branch_type(description: &str) -> &'static str {
    let desc_lower = description.to_lowercase();

    if desc_lower.contains("fix") || desc_lower.contains("bug") || desc_lower.contains("issue") {
        "fix"
    } else if desc_lower.contains("refactor") || desc_lower.contains("optimize") || desc_lower.contains("improve") {
        "refactor"
    } else if desc_lower.contains("docs") || desc_lower.contains("documentation") || desc_lower.contains("readme") {
        "docs"
    } else if desc_lower.contains("test") || desc_lower.contains("testing") {
        "test"
    } else if desc_lower.contains("performance") || desc_lower.contains("perf") || desc_lower.contains("speed") {
        "perf"
    } else if desc_lower.contains("style") || desc_lower.contains("format") || desc_lower.contains("lint") {
        "style"
    } else if desc_lower.contains("config") || desc_lower.contains("setting") {
        "config"
    } else if desc_lower.contains("security") || desc_lower.contains("auth") || desc_lower.contains("permission") {
        "security"
    } else {
        "feat"
    }
}

/// 处理分支命令
pub async fn handle_branch(description: String, create: bool, from_staged: bool) -> Result<()> {
    // 检查是否是一个git仓库
    if !git::check_is_git_repo().await {
        eprintln!("{}", "错误: 当前目录不是一个有效的 Git 仓库。".red());
        return Ok(());
    }

    let llm_client = config::get_llm_client().await?;

    // 获取上下文信息
    let staged_context = if from_staged {
        get_staged_context().await?
    } else {
        String::new()
    };

    // 如果使用 --from-staged 但没有暂存区变更，提示用户
    if from_staged && staged_context.is_empty() {
        println!("{}", "警告: 暂存区没有变更，将仅基于描述生成分支名。".yellow());
    }

    println!("{}", "🤖 正在生成分支名称...".cyan());

    // 生成分支名称
    let branch_name = generate_branch_name(
        llm_client.as_client(),
        &description,
        &staged_context
    ).await?;

    println!("\n{}", "=".repeat(50));
    println!("{} {}", "🌿 建议的分支名称:".green().bold(), branch_name.cyan().bold());
    println!("{}", "=".repeat(50));

    if create {
        // 直接创建并切换分支
        println!("{}", "🚀 正在创建并切换到新分支...".cyan());

        git::run_git_command(&["checkout", "-b", &branch_name])
            .await
            .context("无法创建新分支")?;

        println!("{} {}", "✅ 已创建并切换到分支:".green(), branch_name.cyan().bold());
    } else {
        // 只显示建议，不创建分支
        println!("\n{}", "💡 提示:".yellow());
        println!("  使用以下命令创建并切换到此分支:");
        println!("  {}", format!("git checkout -b {}", branch_name).cyan());
        println!("  或者使用 {} 直接创建:", "matecode branch --create".cyan());
        println!("  {}", format!("matecode branch \"{}\" --create", description).cyan());
    }

    Ok(())
}
