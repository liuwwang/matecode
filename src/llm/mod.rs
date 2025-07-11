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
    async fn call(&self, user_prompt: &str) -> Result<String>;
}

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

    async fn call(&self, user_prompt: &str) -> Result<String> {
        match self {
            LLM::Gemini(c) => c.call(user_prompt).await,
            LLM::OpenAI(c) => c.call(user_prompt).await,
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
    println!("🤖 正在调用 {} 生成提交信息...", client.name());

    let system_prompt = r#"你是一个根据 git diff 内容生成 Conventional Commits 规范的 git commit message 的专家。你的回应应该只包含 commit message，不包含任何额外的解释或引言。commit message 应该是 markdown 格式，以`#`开头。"#;

    let user_prompt = format!(
        r#"请根据以下的 git diff 内容生成一个 git commit message。
<rules>
1. 你是一位专业的 Git commit message 编写专家。
2. 你的回应**只能**包含 commit message 内容，不要有其他任何解释。
3. commit message 必须严格遵守 Conventional Commits 规范。
4. commit message 的 header 部分(第一行)不能超过 50 个字符。
5. commit message 的 subject 应该清晰地描述这次提交的目的。
6. 如果有 scope，请在 type 后用括号附上，例如 `feat(api):`。
7. 根据下面的 `<diff>` 内容，生成一个合适的 commit message。
</rules>
<diff>
{}
</diff>"#,
        diff
    );

    let raw_llm_output = client.call(&user_prompt).await?;

    if let Some(thought) = extract_content(&raw_llm_output, "think") {
        println!(
            "\n🤔 {}{}\n",
            "AI 思考:".bold(),
            format!("\n---\n{}\n---", thought).cyan()
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
