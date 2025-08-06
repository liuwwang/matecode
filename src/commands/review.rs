//! src/commands/review.rs

use crate::commands::linter;
use crate::config;
use crate::git::{analyze_diff, get_staged_diff};
use crate::llm::{parse_prompt_template, LLMClient};
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use termimad::MadSkin;

/// Handles the code review process for staged changes.
pub async fn handle_review(lint: bool) -> Result<()> {
    let diff = get_staged_diff()
        .await
        .context("无法获取用于审查的暂存 git diff。")?;

    if diff.is_empty() {
        println!("{}", "没有需要审查的暂存更改。".yellow());
        return Ok(());
    }

    // If the --lint flag is used, run the linter and capture its output as context for the review.
    let lint_result = if lint {
        println!("{}", "(--lint) 审查前运行 linter...".bold());
        // We pass `false` for `format_sarif` and `ai_enhance` to get the plain text output.
        let result = linter::handle_linter(false, false, None).await?;
        println!("{}", "-".repeat(60));
        result
    } else {
        None
    };

    println!("{}", "🤖 正在生成代码审查...".cyan());
    let llm_client = config::get_llm_client().await?;
    let review =
        generate_diff_code_review(llm_client.as_client(), &diff, lint_result.as_deref()).await?;

    let skin = MadSkin::default();

    println!("\n{}\n", "=".repeat(60));
    skin.print_text(&review);
    println!("\n{}\n", "=".repeat(60));

    Ok(())
}

/// Generates a code review for the given diff using an LLM.
async fn generate_diff_code_review(
    client: &dyn LLMClient,
    diff: &str,
    lint_result: Option<&str>,
) -> Result<String> {
    let template = config::get_prompt_template("review").await?;
    let (system_prompt, user_prompt) = parse_prompt_template(&template)?;

    let analysis = analyze_diff(diff, client.model_config()).await?;

    if analysis.needs_chunking {
        return Err(anyhow!("代码变更过大，暂不支持分块审查。"));
    }

    let progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} AI is analyzing the changes...")
            .unwrap(),
    );
    progress_bar.enable_steady_tick(std::time::Duration::from_millis(100));

    let user_prompt = user_prompt.replace("{diff_content}", &analysis.chunks[0].content);

    let final_prompt = if let Some(lint) = lint_result {
        if !lint.trim().is_empty() {
            let lint_context = format!("\n<lint_results>\n{lint}\n</lint_results>");
            user_prompt.replace("<lint_results></lint_results>", &lint_context)
        } else {
            user_prompt.replace("<lint_results></lint_results>", "")
        }
    } else {
        user_prompt.replace("<lint_results></lint_results>", "")
    };

    let review = client.call(&system_prompt, &final_prompt).await;
    progress_bar.finish_with_message("✓ AI review complete");
    review
}
