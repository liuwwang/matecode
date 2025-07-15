//! src/llm/mod.rs

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub mod gemini;
pub mod openai;

pub use gemini::GeminiClient;
pub use openai::OpenClient;

/// LLM上下文配置
#[derive(Debug, Clone)]
pub struct ContextConfig {
    pub max_tokens: usize,
    pub max_output_tokens: usize,
    pub reserved_tokens: usize, // 为系统prompt和输出预留的token数
}

impl ContextConfig {
    pub fn available_context_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.reserved_tokens)
    }
}

/// The `LLMClient` trait defines the interface for a Large Language Model client.
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// Returns the name of the LLM client.
    fn name(&self) -> &str;
    /// Returns the context configuration for this LLM.
    fn context_config(&self) -> ContextConfig;
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

    fn context_config(&self) -> ContextConfig {
        match self {
            LLM::Gemini(c) => c.context_config(),
            LLM::OpenAI(c) => c.context_config(),
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
    
    // 首先尝试提取标签内容
    if let Some(start_pos) = text.find(&start_tag) {
        let start_byte = start_pos + start_tag.len();
        if let Some(end_pos) = text[start_byte..].find(&end_tag) {
            return Some(text[start_byte..start_byte + end_pos].trim().to_string());
        }
    }
    
    // 如果没有找到标签，对于commit_message，尝试直接返回整个文本
    if tag == "commit_message" {
        // 清理文本，移除可能的markdown格式
        let cleaned = text.trim()
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        
        if !cleaned.is_empty() {
            return Some(cleaned);
        }
    }
    
    None
}

pub async fn generate_commit_message(client: &LLM, diff: &str) -> Result<String> {
    // 获取模型的上下文配置
    let context_config = client.context_config();

    // 分析diff内容
    let analysis = crate::git::analyze_diff(diff, &context_config)?;

    if analysis.needs_chunking {
        generate_chunked_commit_message(client, &analysis).await
    } else {
        generate_single_commit_message(client, &analysis.context, &analysis.chunks[0]).await
    }
}

async fn generate_chunked_commit_message(client: &LLM, analysis: &crate::git::DiffAnalysis) -> Result<String> {
    let mut chunk_summaries = Vec::new();
    
    for (i, chunk) in analysis.chunks.iter().enumerate() {
        // 对于分块，只传递diff内容，不重复传递项目上下文
        let summary = generate_chunk_summary_simple(client, chunk, i + 1, analysis.chunks.len()).await?;
        chunk_summaries.push(summary);
    }
    
    // 基于所有块的摘要生成最终的commit message，这时才传递完整的项目上下文
    generate_final_commit_message(client, &analysis.context, &chunk_summaries).await
}

async fn generate_chunk_summary_simple(
    client: &LLM,
    chunk: &crate::git::DiffChunk,
    chunk_index: usize,
    total_chunks: usize,
) -> Result<String> {
    let system_prompt = r#"你是一个代码变更分析专家。你需要简洁地总结这个代码块的主要变更内容。请用中文回答。"#;

    let user_prompt = format!(
        r#"请分析以下代码变更并生成简洁的中文摘要。

<chunk_info>
这是第 {chunk_index}/{total_chunks} 个代码块
涉及文件: {chunk_files}
</chunk_info>

{diff_content}

请用中文总结这个代码块的主要变更，重点关注功能性改变。
**注意**：只需要描述变更内容，不要生成完整的commit message格式（如feat:、fix:等）。
直接回答变更摘要，不要使用任何标签。

例如：
- 好的回答："添加了用户认证模块和登录功能"
- 不好的回答："feat: 添加用户认证模块"
"#,
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
    let system_prompt = r#"你是一个根据代码变更摘要生成 Conventional Commits 规范的 git commit message 的专家。你的回应应该只包含中文的 commit message，不包含任何额外的解释或引言。"#;

    let formatted_summaries = summaries.iter().enumerate()
        .map(|(i, summary)| format!("{}. {}", i + 1, summary))
        .collect::<Vec<_>>()
        .join("\n");

    let user_prompt = format!(
        r#"请根据以下的项目上下文和代码变更摘要，为我生成一个高质量的、人类可读的中文 git commit message。

**项目上下文**:
{project_tree}

**本次修改影响的文件 ({total_files} 个)**:
{affected_files}

**代码变更摘要 (从多个代码块中提炼)**:
{summaries}

**Commit Message 生成要求**:
<rules>
1.  **角色**: 你是一位资深的软件工程师，现在需要为一系列复杂的变更编写一条清晰、有价值的 commit message。
2.  **目标**: 不要创建一个简单的变更日志 (changelog)。你的目标是写一个**高层次的总结**，解释这次系列变更的**核心目的**和**主要实现**。
3.  **格式**: 严格遵守 Conventional Commits 规范。
    -   **Header**: `type(scope): subject` 格式，`subject` 要高度概括，例如 "重构认证模块以支持多因素认证" 或 "引入历史归档与日报生成功能"。
    -   **Body**:
        -   第一段必须解释 **“为什么”** 进行这些变更。它们解决了什么问题？为项目带来了什么价值？
        -   第二段（可选）可以解释 **“如何”** 实现的，但只描述关键的设计决策和架构变化，**不要**罗列每一个函数或文件的改动。
        -   如果实现细节简单，可以省略 Body。
4.  **禁止项**:
    -   **严禁**直接罗列变更摘要。
    -   **严禁**使用 "本次提交..." 或 "这个PR..." 等 AI 化的句式。
5.  **输出**: 只输出 commit message，不包含任何其他解释。
</rules>

<example_good>
feat(history): 引入提交历史归档与日报生成功能

为了更好地追踪开发进度和自动化生成工作报告，本次引入了提交历史的自动归档机制。

此功能通过 `post-commit` Git 钩子实现，确保只有最终被采纳的 commit 才会被记录。新增的 `report` 命令可以调用 AI 服务，将每日的提交记录智能地汇总成一份结构化的工作日报。
</example_good>

<example_bad>
feat: 新增功能
- 添加 history.rs
- 添加 hook.rs
- 修改 main.rs
- 添加 report 命令
- 添加 archive 命令
</example_bad>
"#,
        project_tree = context.project_tree,
        total_files = context.total_files,
        affected_files = context.affected_files.join(", "),
        summaries = formatted_summaries
    );

    let raw_llm_output = client.call(system_prompt, &user_prompt).await?;

    if let Some(thought) = extract_content(&raw_llm_output, "think") {
        println!(
            "\n🤔 {}{}\n",
            "AI 思考:".bold(),
            format!("\n---\n{thought}\n---").cyan()
        );
    }

    let commit_message = extract_content(&raw_llm_output, "commit_message").unwrap_or_else(|| {
        // 如果无法提取标签，直接使用原始输出
        raw_llm_output.trim().to_string()
    });
    
    if commit_message.is_empty() {
        return Err(anyhow!("LLM 返回了空的 commit message。\n原始输出: {}", raw_llm_output));
    }

    Ok(commit_message)
}

async fn generate_single_commit_message(
    client: &LLM,
    context: &crate::git::ProjectContext,
    chunk: &crate::git::DiffChunk,
) -> Result<String> {
    let system_prompt = r#"你是一个根据 git diff 内容生成 Conventional Commits 规范的 git commit message 的专家。你的回应应该只包含中文的 commit message，不包含任何额外的解释或引言。"#;

    let user_prompt = format!(
        r#"请根据以下的项目上下文和 git diff 内容生成一个中文 git commit message。

<project_context>
{project_tree}

本次修改影响的文件 ({total_files} 个):
{affected_files}
</project_context>

<rules>
1. 你是一位专业的 Git commit message 编写专家，你的目标是生成读起来像人类工程师编写的 commit message。
2. 你的回应**只能**包含中文 commit message 内容，不要有其他任何解释。
3. commit message 必须严格遵守 Conventional Commits 规范，但描述部分使用中文。
4. **Header (第一行)**:
   - `type` 使用英文 (如 feat, fix, chore)。
   - `scope` (可选) 概括变更涉及的模块。
   - `subject` (主题) 必须用清晰的中文简明扼要地描述变更内容，不超过50个字符。
5. **Body (正文, 可选)**:
   - 正文应详细解释 **为什么** 需要这次变更，解决了什么问题。
   - 描述这次变更是 **如何** 实现的，特别是关键的实现思路。
   - 避免使用AI化的、过于正式的语言（例如，不要写 "本次提交新增了..."，而应该更直接地描述）。
   - 如果没有特别复杂的逻辑，可以省略正文。
6. **Footer (页脚, 可选)**:
   - 用于标记重大变更 (BREAKING CHANGE) 或关闭 issue (Closes #123)。
7. **总体要求**:
   - 不要简单地罗列变更的文件和内容，要写出有价值的解释。
   - 基于项目结构和下面的代码变更详情，生成一个高质量的中文 commit message。
   - 直接回答 commit message，不要使用任何 XML 标签。
</rules>

<example_good>
feat(api): 实现用户认证功能

用户认证是系统的核心安全保障。本次提交引入了基于 JWT 的认证机制。
- 使用 `jsonwebtoken` 库生成和验证 token。
- 在 `auth` 中间件中实现 token 校验逻辑。
- 登录成功后，返回带有 token 的响应。
</example_good>

<example_bad>
feat: 添加认证
- 添加了 auth.js
- 修改了 user.js
</example_bad>

{diff_content}"#,
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

    let commit_message = extract_content(&raw_llm_output, "commit_message").unwrap_or_else(|| {
        // 如果无法提取标签，直接使用原始输出
        raw_llm_output.trim().to_string()
    });
    
    if commit_message.is_empty() {
        return Err(anyhow!("LLM 返回了空的 commit message。\n原始输出: {}", raw_llm_output));
    }

    Ok(commit_message)
}

pub async fn generate_daily_report(client: &LLM) -> Result<String> {
    println!("📊 正在收集今日提交记录...");
    let report_data = crate::history::gather_daily_commits()?;

    println!("🧠 正在调用 {} 生成智能日报...", client.name());
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.blue} {msg}")?,
    );
    spinner.set_message("AI正在为您撰写日报，请稍候...");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let report = crate::history::generate_ai_powered_report(client, &report_data).await?;

    spinner.finish_and_clear();

    Ok(report)
}
