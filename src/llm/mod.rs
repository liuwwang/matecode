//! src/llm/mod.rs

use crate::config::{ContextConfig, LLMConfig};
use crate::git::{DiffAnalysis, DiffChunk, ProjectContext};
use anyhow::{anyhow, Result};
use async_recursion::async_recursion;
use async_trait::async_trait;
use chrono::NaiveDate;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::BTreeMap;
use std::time::Duration;

pub mod gemini;
pub mod openai;

#[async_trait]
pub trait LLMClient {
    fn name(&self) -> &str;
    fn context_config(&self) -> ContextConfig;
    async fn call(&self, system_prompt: &str, user_prompt: &str) -> Result<String>;
}

pub enum LLM<'a> {
    OpenAI(openai::OpenClient),
    Gemini(gemini::GeminiClient),
    Other(&'a dyn LLMClient),
}

impl<'a> LLM<'a> {
    pub fn name(&self) -> &str {
        match self {
            LLM::OpenAI(client) => client.name(),
            LLM::Gemini(client) => client.name(),
            LLM::Other(client) => client.name(),
        }
    }

    pub fn context_config(&self) -> ContextConfig {
        match self {
            LLM::OpenAI(client) => client.context_config(),
            LLM::Gemini(client) => client.context_config(),
            LLM::Other(client) => client.context_config(),
        }
    }

    pub async fn call(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        match self {
            LLM::OpenAI(client) => client.call(system_prompt, user_prompt).await,
            LLM::Gemini(client) => client.call(system_prompt, user_prompt).await,
            LLM::Other(client) => client.call(system_prompt, user_prompt).await,
        }
    }
}

pub fn create_llm_client(config: LLMConfig) -> Result<LLM<'static>> {
    match config.provider.as_str() {
        "openai" => Ok(LLM::OpenAI(openai::OpenClient::new(config)?)),
        "gemini" => Ok(LLM::Gemini(gemini::GeminiClient::new(config)?)),
        _ => Err(anyhow!("Unsupported LLM provider: {}", config.provider)),
    }
}

pub async fn generate_commit_message(
    client: &dyn LLMClient,
    diff: &str,
) -> Result<String> {
    let progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::with_template("{spinner} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    progress_bar.enable_steady_tick(Duration::from_millis(100));

    let analysis = crate::git::analyze_diff(diff, &client.context_config())?;

    let commit_message = if analysis.needs_chunking {
        generate_chunked_commit_message(client, &analysis).await?
    } else {
        generate_single_chunk_commit_message(client, &analysis).await?
    };

    progress_bar.finish_with_message("✓ Commit message generated.");
    Ok(commit_message)
}

#[async_recursion]
async fn generate_chunked_commit_message(
    client: &dyn LLMClient,
    analysis: &DiffAnalysis,
) -> Result<String> {
    let mut summaries = Vec::new();

    let progress_bar = ProgressBar::new(analysis.chunks.len() as u64);
    for (i, chunk) in analysis.chunks.iter().enumerate() {
        progress_bar.set_message(format!("Processing chunk {}/{}...", i + 1, analysis.chunks.len()));
        let summary = summarize_chunk(client, &analysis.context, chunk).await?;
        summaries.push(summary);
        progress_bar.inc(1);
    }
    progress_bar.finish_with_message("✓ All chunks summarized.");

    let final_commit_message =
        combine_summaries(client, &analysis.context, &summaries.join("\n\n"))
            .await?;

    Ok(final_commit_message)
}

async fn generate_single_chunk_commit_message(
    client: &dyn LLMClient,
    analysis: &DiffAnalysis,
) -> Result<String> {
    let system_prompt = get_system_prompt();
    let user_prompt = build_user_prompt(&analysis.context, &analysis.chunks[0]);

    let message = client.call(&system_prompt, &user_prompt).await?;
    extract_content(&message, "commit_message")
        .ok_or_else(|| anyhow!("LLM failed to generate a valid commit message from a single chunk."))
}

async fn summarize_chunk(client: &dyn LLMClient, context: &ProjectContext, chunk: &DiffChunk) -> Result<String> {
    let system_prompt = get_summarize_system_prompt();
    let user_prompt = build_summarize_user_prompt(context, chunk);

    let summary = client.call(&system_prompt, &user_prompt).await?;
    extract_content(&summary, "summary")
        .ok_or_else(|| anyhow!("LLM failed to generate a valid summary for a chunk."))
}

async fn combine_summaries(client: &dyn LLMClient, context: &ProjectContext, summaries: &str) -> Result<String> {
    let system_prompt = get_combine_system_prompt();
    let user_prompt = build_combine_user_prompt(context, summaries);

    let message = client.call(&system_prompt, &user_prompt).await?;
    extract_content(&message, "commit_message")
        .ok_or_else(|| anyhow!("LLM failed to combine summaries into a final commit message."))
}

fn get_system_prompt() -> &'static str {
    r#"你是一位专业的 Git commit message 编写专家，你的目标是生成读起来像人类工程师编写的 commit message。你的回应**只能**包含 commit message 内容，不要有其他任何解释。严格遵守 Conventional Commits 规范，但描述部分使用中文。"#
}

fn build_user_prompt(context: &ProjectContext, chunk: &DiffChunk) -> String {
    format!(
        r#"请根据以下的项目上下文和 git diff 内容生成一个中文 git commit message。

<project_context>
{project_tree}

本次修改影响的文件 ({total_files} 个):
{affected_files}
</project_context>

<rules>
1.  **Header (第一行)**:
    -   `type` 使用英文 (如 feat, fix, chore)。
    -   `scope` (可选) 概括变更涉及的模块。
    -   `subject` (主题) 必须用清晰的中文简明扼要地描述变更内容，不超过50个字符。
2.  **Body (正文, 可选)**:
    -   正文应详细解释 **为什么** 需要这次变更，解决了什么问题。
    -   描述这次变更是 **如何** 实现的，特别是关键的实现思路。
    -   避免使用AI化的、过于正式的语言（例如，不要写 "本次提交新增了..."，而应该更直接地描述）。
3.  **输出**: 只输出被 <commit_message> 标签包裹的 commit message。
</rules>

<example_good>
<commit_message>
feat(api): 实现用户认证功能

用户认证是系统的核心安全保障。本次提交引入了基于 JWT 的认证机制。
- 使用 `jsonwebtoken` 库生成和验证 token。
- 在 `auth` 中间件中实现 token 校验逻辑。
</commit_message>
</example_good>

<diff_content>
{diff_content}
</diff_content>"#,
        project_tree = context.project_tree,
        total_files = context.total_files,
        affected_files = context.affected_files.join(", "),
        diff_content = chunk.content
    )
}

fn get_summarize_system_prompt() -> &'static str {
    r#"你是一个代码变更分析专家。你需要简洁地总结这个代码块的主要变更内容。你的回应**只能**包含被 <summary> 标签包裹的摘要。"#
}

fn build_summarize_user_prompt(context: &ProjectContext, chunk: &DiffChunk) -> String {
    format!(
        r#"请分析以下代码变更并生成简洁的中文摘要。

<context>
项目文件数: {total_files}
涉及文件: {chunk_files}
</context>

<diff>
{diff_content}
</diff>

请用中文总结这个代码块的主要变更，重点关注功能性改变。
**注意**：只需要描述变更内容，不要生成完整的commit message格式。

例如:
<summary>
添加了用户认证模块和登录功能，并重构了数据库连接逻辑。
</summary>"#,
        total_files = context.total_files,
        chunk_files = chunk.files.join(", "),
        diff_content = chunk.content
    )
}

fn get_combine_system_prompt() -> &'static str {
    r#"你是一个根据代码变更摘要生成 Conventional Commits 规范的 git commit message 的专家。你的回应应该**只能**包含被 <commit_message> 标签包裹的 commit message，不包含任何额外的解释或引言。"#
}

fn build_combine_user_prompt(context: &ProjectContext, summaries: &str) -> String {
    format!(
        r#"请根据以下的项目上下文和代码变更摘要，为我生成一个高质量的、人类可读的中文 git commit message。

<project_context>
{project_tree}

本次修改影响的文件 ({total_files} 个):
{affected_files}
</project_context>

<summaries>
{summaries}
</summaries>

<rules>
1.  **目标**: 不要创建一个简单的变更日志。你的目标是写一个**高层次的总结**，解释这次系列变更的**核心目的**和**主要实现**。
2.  **格式**: 严格遵守 Conventional Commits 规范。
3.  **输出**: 只输出被 <commit_message> 标签包裹的 commit message。
</rules>

<example>
<commit_message>
feat(history): 引入提交历史归档与日报生成功能

为了更好地追踪开发进度和自动化生成工作报告，本次引入了提交历史的自动归档机制。

此功能通过 `post-commit` Git 钩子实现，确保只有最终被采纳的 commit 才会被记录。新增的 `report` 命令可以调用 AI 服务，将每日的提交记录智能地汇总成一份结构化的工作日报。
</commit_message>
</example>"#,
        project_tree = context.project_tree,
        total_files = context.total_files,
        affected_files = context.affected_files.join(", "),
        summaries = summaries
    )
}

fn extract_content(text: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);
    text.find(&start_tag)
        .and_then(|start| text[start + start_tag.len()..].find(&end_tag).map(|end| (start, end)))
        .map(|(start, end)| text[start + start_tag.len()..start + start_tag.len() + end].trim().to_string())
}

pub async fn generate_report_from_commits(
    client: &dyn LLMClient,
    all_commits: &BTreeMap<String, Vec<String>>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<String> {
    let system_prompt = "You are a senior project manager responsible for writing concise, clear, and insightful work summaries. Your goal is to synthesize a list of raw git commit messages from multiple projects into a unified report that is easy for stakeholders to understand. Group related items, use clear headings, and focus on the accomplishments and outcomes, not just the raw commit messages.";

    let mut commits_context = String::new();
    for (project, commits) in all_commits {
        commits_context.push_str(&format!("\n## Project: {}\n", project));
        for commit in commits {
            commits_context.push_str(&format!("- {}\n", commit.replace('\n', " ")));
        }
    }

    let user_prompt = format!(
        r#"
Please generate a work summary report in Markdown format based on the following commit messages from {} to {}.
The commits are grouped by project.

## Raw Commits:
{}

## Instructions:
1.  **Analyze and Group:** Read through all the commit messages from all projects. Group them into logical categories (e.g., "Feature Development," "Bug Fixes," "Refactoring").
2.  **Summarize Each Group:** For each category, write a high-level summary of the work accomplished. Use bullet points to list the key changes. **Crucially, you must mention which project the change belongs to.**
3.  **Use Clear Headings:** Use Markdown headings (e.g., `### ✨ 新功能`) for each category.
4.  **Focus on Impact:** Rephrase the commit messages to focus on the "what" and "why."
5.  **Language:** The report should be in Chinese.

## Desired Output Format:

### ✨ 新功能
- [项目A] - 实现用户登录和注册功能。
- [项目B] - 新增了数据导出的 API.

### 🐛 问题修复
- [项目A] - 修复了特定场景下闪退的问题。

Please generate the report now.
"#,
        start_date.format("%Y-%m-%d"),
        end_date.format("%Y-%m-%d"),
        commits_context
    );

    client.call(system_prompt, &user_prompt).await
}

pub async fn generate_code_review(client: &dyn LLMClient, diff: &str) -> Result<String> {
    let system_prompt = "You are an expert code reviewer. Your task is to analyze a git diff and provide constructive feedback. Focus on identifying potential bugs, improving code quality, and ensuring best practices are followed. Be clear, concise, and provide actionable suggestions. Structure your review in Markdown format.";

    let user_prompt = format!(
        r#"
Please review the following code changes provided in the git diff format.

## Git Diff:
```diff
{}
```

## Review Guidelines:
1.  **Overall Assessment:** Start with a brief, high-level summary of the changes.
2.  **Identify Issues and Suggestions:** For each file, provide specific feedback. Refer to line numbers where possible.
    -   **[Logic]**: Potential bugs, race conditions, or logic errors.
    -   **[Style]**: Code style, naming conventions, readability.
    -   **[Best Practice]**: Suggestions for using language features or libraries more effectively.
    -   **[Comment]**: Questions or requests for clarification.
3.  **Use Markdown:** Structure the review using headings for each file and bullet points for individual comments.
4.  **Be Constructive:** Frame your feedback positively. The goal is to help improve the code, not to criticize.
5.  **Language**: The review should be in Chinese.

## Example Output:

### `src/main.rs`
- **[Logic] at line 42:** The current logic might not handle empty input gracefully. Consider adding a check at the beginning of the function.
- **[Style] at line 55:** The variable `temp_data` could be renamed to `user_profile` for better clarity.

### `src/utils.rs`
- **[Best Practice] at line 12:** Instead of manually building the path string, consider using `PathBuf::join()` for better cross-platform compatibility.

Please provide your review for the provided diff.
"#,
        diff
    );

    client.call(system_prompt, &user_prompt).await
}
