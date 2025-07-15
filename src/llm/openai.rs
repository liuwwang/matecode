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

const FAKE_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

// --- 客户端实现 ---
pub struct OpenClient {
    api_key: String,
    model_name: String,
    api_base: String,
    client: Client,
}

impl OpenClient {
    pub fn new() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow!("错误：请在 .env 文件中设置 OPENAI_API_KEY"))?;

        let model_name =
            env::var("OPENAI_MODEL_NAME").unwrap_or_else(|_| "gpt-4-turbo".to_string());

        let api_base = env::var("OPENAI_API_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string());

        // 强制从环境变量构建代理
        let proxy_url = env::var("ALL_PROXY").or_else(|_| env::var("HTTPS_PROXY")).ok();
        let client = match proxy_url {
            Some(url) => {
                let proxy = reqwest::Proxy::all(&url)?;
                Client::builder()
                    .proxy(proxy)
                    .user_agent(FAKE_USER_AGENT)
                    .build()?
            }
            None => Client::builder().user_agent(FAKE_USER_AGENT).build()?,
        };

        Ok(Self {
            api_key,
            model_name,
            api_base,
            client,
        })
    }
}

#[async_trait::async_trait]
impl LLMClient for OpenClient {
    fn name(&self) -> &str {
        "OpenAI"
    }

    fn context_config(&self) -> super::ContextConfig {
        // 根据不同的OpenAI模型返回不同的配置
        match self.model_name.as_str() {
            "gpt-4-turbo" | "gpt-4-turbo-2024-04-09" => super::ContextConfig {
                max_tokens: 128_000,
                max_output_tokens: 4_096,
                reserved_tokens: 2_000, // 为系统prompt和输出预留
            },
            "gpt-4" | "gpt-4-0613" => super::ContextConfig {
                max_tokens: 8_192,
                max_output_tokens: 4_096,
                reserved_tokens: 1_500,
            },
            "gpt-3.5-turbo" | "gpt-3.5-turbo-0125" => super::ContextConfig {
                max_tokens: 16_385,
                max_output_tokens: 4_096,
                reserved_tokens: 1_500,
            },
            // 默认配置（保守估计）
            _ => super::ContextConfig {
                max_tokens: 8_192,
                max_output_tokens: 4_096,
                reserved_tokens: 1_500,
            },
        }
    }

    async fn call(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        let request_payload = OpenAIRequest {
            model: &self.model_name,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user",
                    content: user_prompt,
                },
            ],
            temperature: 0.6,
        };

        let res = self
            .client
            .post(&self.api_base)
            .bearer_auth(&self.api_key)
            .json(&request_payload)
            .send()
            .await?;

        let res_status = res.status();

        if res_status.is_success() {
            let response = res.json::<OpenAIResponse>().await?;
            if let Some(first_choice) = response.choices.first() {
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
