//! src/llm/mod.rs

use anyhow::{anyhow, Result};
use async_trait::async_trait;

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

fn extract_from_xml(text: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);
    text.find(&start_tag)
        .and_then(|start| {
            text[start + start_tag.len()..]
                .find(&end_tag)
                .map(|end| text[start + start_tag.len()..start + start_tag.len() + end].to_string())
        })
        .map(|s| s.trim().to_string())
}

pub async fn generate_commit_message(client: &LLM, diff: &str) -> Result<String> {
    println!(
        "🤖 Calling {} to generate commit message...",
        client.name()
    );

    let user_prompt = format!(
        r#"Please generate a commit message based on the following git diff.
<rules>
1. You are an expert at writing Git commit messages.
2. Strictly follow the Conventional Commits specification.
3. Your entire response must be only the commit message.
4. Do not include any markdown formatting (like ```).
5. Enclose the final commit message completely within a <commit_message> XML tag.
</rules>
<example>
<commit_message>
feat(api): add user authentication endpoint

Implements JWT login and registration for users, including password hashing and token generation.
</commit_message>
</example>

Diff:
```diff
{}
```
"#,
        diff
    );

    let raw_llm_output = client.call(&user_prompt).await?;

    let commit_message = extract_from_xml(&raw_llm_output, "commit_message").ok_or_else(|| {
        anyhow!(
            "Could not extract <commit_message> tag from LLM response.\nRaw output: {}",
            raw_llm_output
        )
    })?;

    Ok(commit_message)
} 