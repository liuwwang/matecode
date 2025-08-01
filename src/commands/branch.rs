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
async fn generate_branch_name(
    client: &dyn LLMClient,
    description: &str,
    staged_context: &str,
) -> Result<String> {
    let template = get_prompt_template("branch").await?;
    let (system_prompt, user_prompt) = parse_prompt_template(&template)?;

    let user_prompt = build_branch_user_prompt(&user_prompt, description, staged_context);

    let response = client.call(&system_prompt, &user_prompt).await?;

    extract_branch_name(&response).ok_or_else(|| anyhow!("无法从 LLM 响应中提取有效的分支名称"))
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
        println!(
            "{}",
            "警告: 暂存区没有变更，将仅基于描述生成分支名。".yellow()
        );
    }

    println!("{}", "🤖 正在生成分支名称...".cyan());

    // 生成分支名称
    let branch_name =
        generate_branch_name(llm_client.as_client(), &description, &staged_context).await?;

    println!("\n{}", "=".repeat(50));
    println!(
        "{} {}",
        "🌿 建议的分支名称:".green().bold(),
        branch_name.cyan().bold()
    );
    println!("{}", "=".repeat(50));

    if create {
        // 直接创建并切换分支
        println!("{}", "🚀 正在创建并切换到新分支...".cyan());

        git::run_git_command(&["checkout", "-b", &branch_name])
            .await
            .context("无法创建新分支")?;

        println!(
            "{} {}",
            "✅ 已创建并切换到分支:".green(),
            branch_name.cyan().bold()
        );
    } else {
        // 只显示建议，不创建分支
        println!("\n{}", "💡 提示:".yellow());
        println!("  使用以下命令创建并切换到此分支:");
        println!("  {}", format!("git checkout -b {}", branch_name).cyan());
        println!("  或者使用 {} 直接创建:", "matecode branch --create".cyan());
        println!(
            "  {}",
            format!("matecode branch \"{}\" --create", description).cyan()
        );
    }

    Ok(())
}
