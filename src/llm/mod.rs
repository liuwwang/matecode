//! src/llm/mod.rs

use crate::config::{Config, ModelConfig, get_prompt_template};
use crate::git::{DiffAnalysis, DiffChunk, ProjectContext};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

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
            let openai_config = config
                .llm
                .openai
                .as_ref()
                .ok_or_else(|| anyhow!("OpenAI 配置未找到"))?;
            Ok(LLM::OpenAI(openai::OpenAIClient::new(openai_config)?))
        }
        "gemini" => {
            let gemini_config = config
                .llm
                .gemini
                .as_ref()
                .ok_or_else(|| anyhow!("Gemini 配置未找到"))?;
            Ok(LLM::Gemini(gemini::GeminiClient::new(gemini_config)?))
        }
        _ => Err(anyhow!("不支持的 LLM 提供商: {}", config.provider)),
    }
}

pub async fn generate_commit_message(client: &dyn LLMClient, diff: &str) -> Result<String> {
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
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    progress_bar.set_length(analysis.chunks.len() as u64);
    progress_bar.set_position(0);
    progress_bar.set_message("Summarizing chunks...");

    let summaries_stream = stream::iter(
        analysis
            .chunks
            .iter()
            .map(|chunk| async move { summarize_chunk(client, &analysis.context, chunk).await }),
    );

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

async fn summarize_chunk(
    client: &dyn LLMClient,
    context: &ProjectContext,
    chunk: &DiffChunk,
) -> Result<String> {
    let template = get_prompt_template("summarize").await?;
    let (system_prompt, user_prompt) = parse_prompt_template(&template)?;

    let user_prompt = build_summarize_user_prompt(&user_prompt, context, chunk);

    let summary = client.call(&system_prompt, &user_prompt).await?;
    extract_content(&summary, "summary").ok_or_else(|| anyhow!("LLM 无法为代码块生成有效的摘要。"))
}

async fn combine_summaries(
    client: &dyn LLMClient,
    context: &ProjectContext,
    summaries: &str,
) -> Result<String> {
    let template = get_prompt_template("combine").await?;
    let (system_prompt, user_prompt) = parse_prompt_template(&template)?;

    let user_prompt = build_combine_user_prompt(&user_prompt, context, summaries);

    let message = client.call(&system_prompt, &user_prompt).await?;
    extract_content(&message, "commit_message")
        .ok_or_else(|| anyhow!("LLM 无法将摘要合并为最终的提交信息。"))
}

// --- Helper Functions ---
pub(crate) fn parse_prompt_template(template: &str) -> Result<(String, String)> {
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

    Ok((
        system_prompt.trim().to_string(),
        user_prompt.trim().to_string(),
    ))
}

fn build_user_prompt(template: &str, context: &ProjectContext, chunk: &DiffChunk) -> String {
    template
        .replace("{project_tree}", &context.project_tree)
        .replace("{total_files}", &context.total_files.to_string())
        .replace("{affected_files}", &context.affected_files.join(", "))
        .replace("{diff_content}", &chunk.content)
}

fn build_summarize_user_prompt(
    template: &str,
    context: &ProjectContext,
    chunk: &DiffChunk,
) -> String {
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

fn extract_content(text: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{tag}>");
    let end_tag = format!("</{tag}>");

    let start = text.find(&start_tag)? + start_tag.len();
    let end = text.find(&end_tag)?;

    Some(text[start..end].trim().to_string())
}
