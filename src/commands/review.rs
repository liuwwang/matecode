use crate::commands::linter::handle_linter;
use crate::config;
use crate::config::get_prompt_template;
use crate::git::{get_staged_diff, DiffChunk, ProjectContext};
use crate::llm::{parse_prompt_template, LLMClient};
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use termimad::MadSkin;
use tokio::time;

fn build_review_user_prompt(
    template: &str,
    context: &ProjectContext,
    chunk: &DiffChunk,
    lint_result: Option<&str>,
) -> String {
    let base_prompt = template
        .replace("{project_tree}", &context.project_tree)
        .replace("{total_files}", &context.total_files.to_string())
        .replace("{affected_files}", &context.affected_files.join(", "))
        .replace("{diff_content}", &chunk.content);

    if let Some(lint) = lint_result {
        if !lint.trim().is_empty() {
            let lint_context = format!("<lint_results>\n{lint}\n</lint_results>");
            return base_prompt.replace("<lint_results></lint_results>", &lint_context);
        }
    }

    // 如果没有 lint 结果，就移除占位符
    base_prompt.replace("<lint_results></lint_results>", "")
}

async fn generate_code_review(
    client: &dyn LLMClient,
    diff: &str,
    lint_result: Option<&str>,
) -> Result<String> {
    let template = get_prompt_template("review").await?;
    let (system_prompt, user_prompt) = parse_prompt_template(&template)?;

    let analysis = crate::git::analyze_diff(diff, client.model_config()).await?;

    // 创建进度条
    let progress_bar = ProgressBar::new(analysis.context.affected_files.len() as u64);
    progress_bar
        .set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} 正在审查: {msg}")
        .unwrap()
        .progress_chars("#>-"));

    // 显示正在审查的文件
    for (i, file) in analysis.context.affected_files.iter().enumerate() {
        progress_bar.set_position(i as u64);
        progress_bar.set_message(file.clone());

        // 添加一个小延迟让用户看到进度
        time::sleep(Duration::from_millis(100)).await;
    }

    progress_bar.set_message("生成审查报告...");

    // For now, code review only supports single chunks.
    // We can extend this later to support chunking like commit messages.
    if analysis.needs_chunking {
        progress_bar.finish_with_message("✗ 代码变更过大，暂不支持审查");
        return Err(anyhow!("暂不支持审查大型代码变更。"));
    }

    let user_prompt = build_review_user_prompt(
        &user_prompt,
        &analysis.context,
        &analysis.chunks[0],
        lint_result,
    );

    let review = client.call(&system_prompt, &user_prompt).await?;

    progress_bar.finish_with_message("✓ 代码审查完成");

    Ok(review)
}

pub async fn handle_review(lint: bool) -> Result<()> {
    let diff = get_staged_diff()
        .await
        .context("无法获取用于审查的暂存 git diff。")?;

    if diff.is_empty() {
        println!("{}", "没有需要审查的暂存更改。".yellow());
        return Ok(());
    }

    let lint_result = if lint {
        println!("{}", "(--lint) 审查前运行 linter...".bold());
        let result = handle_linter(false).await?;
        println!("{}", "-".repeat(60));
        result
    } else {
        None
    };

    let llm_client = config::get_llm_client().await?;
    let review =
        generate_code_review(llm_client.as_client(), &diff, lint_result.as_deref()).await?;

    let skin = MadSkin::default();

    println!("\n{}\n", "=".repeat(60));
    skin.print_text(&review);
    println!("\n{}\n", "=".repeat(60));

    Ok(())
}
