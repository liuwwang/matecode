//! src/llm/openai.rs
use super::LLMClient;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

// --- 数据结构定义 (适配 OpenAI/vLLM) ---
#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct OpenAIRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize)]
struct MessageContent {
    content: String,
}

// --- 客户端实现 ---
pub struct OpenClient {
    api_key: String,
    model_name: String,
    api_base: String,
}

impl OpenClient {
    pub fn new() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow!("错误：请在 .env 文件中设置 OPENAI_API_KEY"))?;

        let model_name =
            env::var("OPENAI_MODEL_NAME").unwrap_or_else(|_| "gpt-4-turbo".to_string());

        let api_base = env::var("OPENAI_API_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string());

        Ok(Self {
            api_key,
            model_name,
            api_base,
        })
    }
}

#[async_trait::async_trait]
impl LLMClient for OpenClient {
    fn name(&self) -> &str {
        "OpenAI"
    }

    async fn call(&self, user_prompt: &str) -> Result<String> {
        let client = Client::new();

        let request_payload = OpenAIRequest {
            model: &self.model_name,
            messages: vec![ChatMessage {
                role: "user",
                content: user_prompt,
            }],
            temperature: 0.6,
        };

        let res = client
            .post(&self.api_base)
            .bearer_auth(&self.api_key)
            .json(&request_payload)
            .send()
            .await?;

        let res_status = res.status();

        if res_status.is_success() {
            let response = res.json::<OpenAIResponse>().await?;
            if let Some(first_choice) = response.choices.get(0) {
                Ok(first_choice.message.content.trim().to_string())
            } else {
                Err(anyhow::anyhow!("API 调用成功，但返回的 'choices' 数组为空"))
            }
        } else {
            let error_body = res.text().await?;

            Err(anyhow!(
                "调用 OpenAI 兼容 API 失败: {}\n响应体: {}",
                res_status,
                error_body
            ))
        }
    }
}
