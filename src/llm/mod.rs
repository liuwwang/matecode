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
    println!("🤖 正在调用 {} 生成提交信息...", client.name());

    let user_prompt = format!(
        r#"请根据以下的 git diff 内容生成一个 git commit message。
<rules>
1. 你是一位专业的 Git commit message 编写专家。
2. 严格遵守 Conventional Commits 规范。
3. 你的所有输出必须严格只有 commit message，并且必须是中文。
4. 在开始生成 commit message 之前，你可以先在 <think> XML 标签中进行思考。这部分是可选的。
5. 不要包含任何 markdown 格式（例如 ```）。
6. 将最终的 commit message 完全包裹在 <commit_message> XML 标签内。
</rules>
<example>
<think>
用户修改了 README 文件，添加了关于项目安装和使用的说明。这是一个文档类型的变更，不涉及代码功能。所以我应该使用 'docs' 作为类型。
</think>
<commit_message>
docs(readme): 完善项目说明

增加了安装和使用方法的详细介绍。
</commit_message>
</example>

差异(Diff):
```diff
{}
```
"#,
        diff
    );

    let raw_llm_output = client.call(&user_prompt).await?;

    if let Some(thought) = extract_from_xml(&raw_llm_output, "think") {
        println!(
            "\n🤔 {}{}\n",
            "AI 思考:".bold(),
            format!("\n---\n{}\n---", thought).cyan()
        );
    }

    let commit_message = extract_from_xml(&raw_llm_output, "commit_message").ok_or_else(|| {
        anyhow!(
            "无法从 LLM 响应中提取 <commit_message> 标签。\n原始输出: {}",
            raw_llm_output
        )
    })?;

    Ok(commit_message)
}
