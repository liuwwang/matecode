//! src/llm/mod.rs

use crate::config::{Config, ModelConfig, get_prompt_template};
use crate::git::{DiffAnalysis, DiffChunk, ProjectContext};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::NaiveDate;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::time;

pub mod gemini;
pub mod openai;

#[async_trait]
pub trait LLMClient: Send + Sync {
    fn model_config(&self) -> &ModelConfig;
    async fn call(&self, system_prompt: &str, user_prompt: &str) -> Result<String>;
}

pub enum LLM {
    OpenAI(openai::OpenAIClient),
    Gemini(gemini::GeminiClient),
}

impl LLM {
    pub fn as_client(&self) -> &dyn LLMClient {
        match self {
            LLM::OpenAI(client) => client,
            LLM::Gemini(client) => client,
        }
    }
}

pub fn create_llm_client(config: &Config) -> Result<LLM> {
    match config.provider.as_str() {
        "openai" => {
            let openai_config = config.llm.openai.as_ref()
                .ok_or_else(|| anyhow!("OpenAI 配置未找到"))?;
            Ok(LLM::OpenAI(openai::OpenAIClient::new(openai_config)?))
        }
        "gemini" => {
            let gemini_config = config.llm.gemini.as_ref()
                .ok_or_else(|| anyhow!("Gemini 配置未找到"))?;
            Ok(LLM::Gemini(gemini::GeminiClient::new(gemini_config)?))
        }
        _ => Err(anyhow!("不支持的 LLM 提供商: {}", config.provider)),
    }
}

pub async fn generate_commit_message(
    client: &dyn LLMClient,
    diff: &str,
) -> Result<String> {
    let progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::with_template("{spinner:.green} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    progress_bar.enable_steady_tick(Duration::from_millis(100));
    progress_bar.set_message("Analyzing changes...");

    let analysis = crate::git::analyze_diff(diff, client.model_config()).await?;

    let commit_message = if analysis.needs_chunking {
        generate_chunked_commit_message(client, &analysis, &progress_bar).await?
    } else {
        progress_bar.set_message("Generating commit message...");
        generate_single_chunk_commit_message(client, &analysis).await?
    };

    progress_bar.finish_with_message("✓ Commit message generated.");
    Ok(commit_message)
}

async fn generate_chunked_commit_message(
    client: &dyn LLMClient,
    analysis: &DiffAnalysis,
    progress_bar: &ProgressBar,
) -> Result<String> {
    progress_bar.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
        .unwrap()
        .progress_chars("#>-"));
    progress_bar.set_length(analysis.chunks.len() as u64);
    progress_bar.set_position(0);
    progress_bar.set_message("Summarizing chunks...");

    let summaries_stream = stream::iter(analysis.chunks.iter().map(|chunk| {
        async move {
            summarize_chunk(client, &analysis.context, chunk).await
        }
    }));
    
    // Process chunks concurrently.
    let mut summaries = Vec::with_capacity(analysis.chunks.len());
    let mut buffered_stream = summaries_stream.buffer_unordered(4); // Concurrency level: 4

    while let Some(result) = buffered_stream.next().await {
        summaries.push(result?);
        progress_bar.inc(1);
    }
    
    progress_bar.set_style(ProgressStyle::with_template("{spinner:.green} {msg}").unwrap());
    progress_bar.set_message("Combining summaries...");

    combine_summaries(client, &analysis.context, &summaries.join("\n\n")).await
}

async fn generate_single_chunk_commit_message(
    client: &dyn LLMClient,
    analysis: &DiffAnalysis,
) -> Result<String> {
    let template = get_prompt_template("commit").await?;
    let (system_prompt, user_prompt) = parse_prompt_template(&template)?;
    
    let user_prompt = build_user_prompt(&user_prompt, &analysis.context, &analysis.chunks[0]);

    let message = client.call(&system_prompt, &user_prompt).await?;
    extract_content(&message, "commit_message")
        .ok_or_else(|| anyhow!("LLM 无法从单个块生成有效的提交信息。"))
}

async fn summarize_chunk(client: &dyn LLMClient, context: &ProjectContext, chunk: &DiffChunk) -> Result<String> {
    let template = get_prompt_template("summarize").await?;
    let (system_prompt, user_prompt) = parse_prompt_template(&template)?;
    
    let user_prompt = build_summarize_user_prompt(&user_prompt, context, chunk);

    let summary = client.call(&system_prompt, &user_prompt).await?;
    extract_content(&summary, "summary")
        .ok_or_else(|| anyhow!("LLM 无法为代码块生成有效的摘要。"))
}

async fn combine_summaries(client: &dyn LLMClient, context: &ProjectContext, summaries: &str) -> Result<String> {
    let template = get_prompt_template("combine").await?;
    let (system_prompt, user_prompt) = parse_prompt_template(&template)?;
    
    let user_prompt = build_combine_user_prompt(&user_prompt, context, summaries);

    let message = client.call(&system_prompt, &user_prompt).await?;
    extract_content(&message, "commit_message")
        .ok_or_else(|| anyhow!("LLM 无法将摘要合并为最终的提交信息。"))
}

pub async fn generate_report_from_commits(
    client: &dyn LLMClient,
    commits: &BTreeMap<String, Vec<String>>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<String> {
    let template = get_prompt_template("report").await?;
    let (system_prompt, user_prompt) = parse_prompt_template(&template)?;
    
    let commits_text = format_commits_for_report(commits);
    let user_prompt = user_prompt
        .replace("{start_date}", &start_date.to_string())
        .replace("{end_date}", &end_date.to_string())
        .replace("{commits}", &commits_text);

    client.call(&system_prompt, &user_prompt).await
}

pub async fn generate_code_review(
    client: &dyn LLMClient,
    diff: &str,
    lint_result: Option<&str>,
) -> Result<String> {
    let template = get_prompt_template("review").await?;
    let (system_prompt, user_prompt) = parse_prompt_template(&template)?;
    
    let analysis = crate::git::analyze_diff(diff, client.model_config()).await?;
    
    // 创建进度条
    let progress_bar = ProgressBar::new(analysis.context.affected_files.len() as u64);
    progress_bar.set_style(ProgressStyle::default_bar()
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
    
    let user_prompt = build_review_user_prompt(&user_prompt, &analysis.context, &analysis.chunks[0], lint_result);

    let review = client.call(&system_prompt, &user_prompt).await?;
    
    progress_bar.finish_with_message("✓ 代码审查完成");
    
    Ok(review)
}


// --- Helper Functions ---

fn parse_prompt_template(template: &str) -> Result<(String, String)> {
    let mut system_prompt = String::new();
    let mut user_prompt = String::new();
    let mut current_section = "";
    
    for line in template.lines() {
        let trimmed_line = line.trim();
        if trimmed_line == "[system]" {
            current_section = "system";
        } else if trimmed_line == "[user]" {
            current_section = "user";
        } else if !trimmed_line.is_empty() {
            match current_section {
                "system" => system_prompt.push_str(line),
                "user" => user_prompt.push_str(line),
                _ => {}
            }
        }
    }
    
    Ok((system_prompt.trim().to_string(), user_prompt.trim().to_string()))
}

fn build_user_prompt(template: &str, context: &ProjectContext, chunk: &DiffChunk) -> String {
    template
        .replace("{project_tree}", &context.project_tree)
        .replace("{total_files}", &context.total_files.to_string())
        .replace("{affected_files}", &context.affected_files.join(", "))
        .replace("{diff_content}", &chunk.content)
}

fn build_summarize_user_prompt(template: &str, context: &ProjectContext, chunk: &DiffChunk) -> String {
    template
        .replace("{total_files}", &context.total_files.to_string())
        .replace("{chunk_files}", &chunk.files.join(", "))
        .replace("{diff_content}", &chunk.content)
}

fn build_combine_user_prompt(template: &str, context: &ProjectContext, summaries: &str) -> String {
    template
        .replace("{project_tree}", &context.project_tree)
        .replace("{total_files}", &context.total_files.to_string())
        .replace("{affected_files}", &context.affected_files.join(", "))
        .replace("{summaries}", summaries)
}

fn build_review_user_prompt(template: &str, context: &ProjectContext, chunk: &DiffChunk, lint_result: Option<&str>) -> String {
    let base_prompt = template
        .replace("{project_tree}", &context.project_tree)
        .replace("{total_files}", &context.total_files.to_string())
        .replace("{affected_files}", &context.affected_files.join(", "))
        .replace("{diff_content}", &chunk.content);
    
    if let Some(lint) = lint_result {
        if !lint.trim().is_empty() {
            let lint_context = format!(
                "<lint_results>\n{lint}\n</lint_results>"
            );
            return base_prompt.replace("<lint_results></lint_results>", &lint_context);
        }
    }
    
    // 如果没有 lint 结果，就移除占位符
    base_prompt.replace("<lint_results></lint_results>", "")
}

fn format_commits_for_report(commits: &BTreeMap<String, Vec<String>>) -> String {
    let mut report = String::new();
    for (author, messages) in commits {
        report.push_str(&format!("- **{author}**\n"));
        for msg in messages {
            report.push_str(&format!("  - {msg}\n"));
        }
        report.push_str("\n");
    }
    report
}

fn extract_content(text: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{tag}>");
    let end_tag = format!("</{tag}>");
    
    let start = text.find(&start_tag)? + start_tag.len();
    let end = text.find(&end_tag)?;
    
    Some(text[start..end].trim().to_string())
}
