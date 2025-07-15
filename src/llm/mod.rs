//! src/llm/mod.rs

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use colored::Colorize;

pub mod gemini;
pub mod openai;

pub use gemini::GeminiClient;
pub use openai::OpenClient;

/// The `LLMClient` trait defines the interface for a Large Language Model client.
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// Returns the name of the LLM client.
    fn name(&self) -> &str;
    /// Calls the LLM with a user prompt and returns the generated response.
    async fn call(&self, system_prompt: &str, user_prompt: &str) -> Result<String>;
}

#[allow(clippy::upper_case_acronyms)]
pub enum LLM {
    Gemini(GeminiClient),
    OpenAI(OpenClient),
}

#[async_trait]
impl LLMClient for LLM {
    fn name(&self) -> &str {
        match self {
            LLM::Gemini(c) => c.name(),
            LLM::OpenAI(c) => c.name(),
        }
    }

    async fn call(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        match self {
            LLM::Gemini(c) => c.call(system_prompt, user_prompt).await,
            LLM::OpenAI(c) => c.call(system_prompt, user_prompt).await,
        }
    }
}

pub fn extract_content(text: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{tag}>");
    let end_tag = format!("</{tag}>");
    let start_byte = text.find(&start_tag)? + start_tag.len();
    text[start_byte..]
        .find(&end_tag)
        .map(|end| text[start_byte..start_byte + end].to_string())
        .map(|s| s.trim().to_string())
}

pub async fn generate_commit_message(client: &LLM, diff: &str) -> Result<String> {
    // 分析diff内容
    let analysis = crate::git::analyze_diff(diff)?;
    
    if analysis.needs_chunking {
        println!("⚠️  Diff内容较大 ({} 字符)，将分块处理 ({} 个块)", 
                 analysis.total_size, analysis.chunks.len());
        generate_chunked_commit_message(client, &analysis).await
    } else {
        println!("🤖 正在调用 {} 生成提交信息...", client.name());
        generate_single_commit_message(client, &analysis.context, &analysis.chunks[0]).await
    }
}

async fn generate_chunked_commit_message(client: &LLM, analysis: &crate::git::DiffAnalysis) -> Result<String> {
    let mut chunk_summaries = Vec::new();
    
    for (i, chunk) in analysis.chunks.iter().enumerate() {
        println!("🔄 正在处理第 {}/{} 个块 ({} 字符)...", 
                 i + 1, analysis.chunks.len(), chunk.size);
        
        let summary = generate_chunk_summary(client, &analysis.context, chunk, i + 1, analysis.chunks.len()).await?;
        chunk_summaries.push(summary);
    }
    
    // 基于所有块的摘要生成最终的commit message
    println!("🔄 正在生成最终的提交信息...");
    generate_final_commit_message(client, &analysis.context, &chunk_summaries).await
}

async fn generate_chunk_summary(
    client: &LLM,
    context: &crate::git::ProjectContext,
    chunk: &crate::git::DiffChunk,
    chunk_index: usize,
    total_chunks: usize,
) -> Result<String> {
    let system_prompt = r#"你是一个代码变更分析专家。你需要简洁地总结这个代码块的主要变更内容。"#;

    let user_prompt = format!(
        r#"请分析以下代码变更并生成简洁的摘要。

<project_context>
{project_tree}

本次修改影响的文件 ({total_files} 个):
{affected_files}
</project_context>

<chunk_info>
这是第 {chunk_index}/{total_chunks} 个代码块
涉及文件: {chunk_files}
</chunk_info>

<diff>
{diff_content}
</diff>

请用1-2句话总结这个代码块的主要变更，重点关注功能性改变。"#,
        project_tree = context.project_tree,
        total_files = context.total_files,
        affected_files = context.affected_files.join(", "),
        chunk_index = chunk_index,
        total_chunks = total_chunks,
        chunk_files = chunk.files.join(", "),
        diff_content = chunk.content
    );

    client.call(system_prompt, &user_prompt).await
}

async fn generate_final_commit_message(
    client: &LLM,
    context: &crate::git::ProjectContext,
    summaries: &[String],
) -> Result<String> {
    let system_prompt = r#"你是一个根据代码变更摘要生成 Conventional Commits 规范的 git commit message 的专家。你的回应应该只包含 commit message，不包含任何额外的解释或引言。commit message 应该是 markdown 格式，以`#`开头。"#;

    let combined_summaries = summaries.join("\n\n");
    
    let user_prompt = format!(
        r#"请根据以下的项目上下文和代码变更摘要生成一个 git commit message。

<project_context>
{project_tree}

本次修改影响的文件 ({total_files} 个):
{affected_files}
</project_context>

<change_summaries>
{summaries}
</change_summaries>

<rules>
1. 你是一位专业的 Git commit message 编写专家。
2. 你的回应**只能**包含 commit message 内容，不要有其他任何解释。
3. commit message 必须严格遵守 Conventional Commits 规范。
4. commit message 的 header 部分(第一行)不能超过 50 个字符。
5. commit message 的 subject 应该清晰地描述这次提交的目的。
6. 如果有 scope，请在 type 后用括号附上，例如 `feat(api):`。
7. 基于项目结构和变更摘要，生成一个合适的 commit message。
8. 如果变更涉及多个功能模块，选择最主要的变更作为commit message的主题。
</rules>"#,
        project_tree = context.project_tree,
        total_files = context.total_files,
        affected_files = context.affected_files.join(", "),
        summaries = combined_summaries
    );

    let raw_llm_output = client.call(system_prompt, &user_prompt).await?;

    if let Some(thought) = extract_content(&raw_llm_output, "think") {
        println!(
            "\n🤔 {}{}\n",
            "AI 思考:".bold(),
            format!("\n---\n{thought}\n---").cyan()
        );
    }

    let commit_message = extract_content(&raw_llm_output, "commit_message").ok_or_else(|| {
        anyhow!(
            "无法从 LLM 响应中提取 <commit_message> 标签。\n原始输出: {}",
            raw_llm_output
        )
    })?;

    Ok(commit_message)
}

async fn generate_single_commit_message(
    client: &LLM,
    context: &crate::git::ProjectContext,
    chunk: &crate::git::DiffChunk,
) -> Result<String> {
    let system_prompt = r#"你是一个根据 git diff 内容生成 Conventional Commits 规范的 git commit message 的专家。你的回应应该只包含 commit message，不包含任何额外的解释或引言。commit message 应该是 markdown 格式，以`#`开头。"#;

    let user_prompt = format!(
        r#"请根据以下的项目上下文和 git diff 内容生成一个 git commit message。

<project_context>
{project_tree}

本次修改影响的文件 ({total_files} 个):
{affected_files}
</project_context>

<rules>
1. 你是一位专业的 Git commit message 编写专家。
2. 你的回应**只能**包含 commit message 内容，不要有其他任何解释。
3. commit message 必须严格遵守 Conventional Commits 规范。
4. commit message 的 header 部分(第一行)不能超过 50 个字符。
5. commit message 的 subject 应该清晰地描述这次提交的目的。
6. 如果有 scope，请在 type 后用括号附上，例如 `feat(api):`。
7. 基于项目结构和下面的 `<diff>` 内容，生成一个合适的 commit message。
</rules>

<diff>
{diff_content}
</diff>"#,
        project_tree = context.project_tree,
        total_files = context.total_files,
        affected_files = context.affected_files.join(", "),
        diff_content = chunk.content
    );

    let raw_llm_output = client.call(system_prompt, &user_prompt).await?;

    if let Some(thought) = extract_content(&raw_llm_output, "think") {
        println!(
            "\n🤔 {}{}\n",
            "AI 思考:".bold(),
            format!("\n---\n{thought}\n---").cyan()
        );
    }

    let commit_message = extract_content(&raw_llm_output, "commit_message").ok_or_else(|| {
        anyhow!(
            "无法从 LLM 响应中提取 <commit_message> 标签。\n原始输出: {}",
            raw_llm_output
        )
    })?;

    Ok(commit_message)
}
