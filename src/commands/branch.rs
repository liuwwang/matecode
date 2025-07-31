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

/// 智能生成分支名称（简化版，要求输入英文描述）
pub fn generate_smart_branch_name(description: &str) -> String {
    // 验证输入是否包含中文字符
    if contains_chinese(description) {
        eprintln!("⚠️  警告: 分支描述应使用英文，当前输入包含中文字符");
        eprintln!("💡 建议: 请使用英文描述，例如 'add user authentication' 而不是'添加用户认证'");
    }

    // 清理和格式化分支名称
    let sanitized = description
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-')
        .collect::<String>()
        .split_whitespace()
        .take(4)  // 限制为4个词以获得更好的描述性
        .collect::<Vec<_>>()
        .join("-");

    // 如果清理后为空，使用默认名称
    if sanitized.is_empty() {
        return "feature/new-feature".to_string();
    }

    // 检测分支类型并添加前缀
    let prefix = determine_branch_type(description);

    format!("{}/{}", prefix, sanitized)
}

/// 检测字符串是否包含中文字符
fn contains_chinese(text: &str) -> bool {
    text.chars().any(|c| {
        let code = c as u32;
        // 中文字符的 Unicode 范围
        (0x4E00..=0x9FFF).contains(&code) || // CJK 统一汉字
        (0x3400..=0x4DBF).contains(&code) || // CJK 扩展 A
        (0x20000..=0x2A6DF).contains(&code) || // CJK 扩展 B
        (0x2A700..=0x2B73F).contains(&code) || // CJK 扩展 C
        (0x2B740..=0x2B81F).contains(&code) || // CJK 扩展 D
        (0x2B820..=0x2CEAF).contains(&code) // CJK 扩展 E
    })
}

// 翻译函数已完全移除 - 现在要求直接使用英文描述

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
