//! src/llm/openai.rs
use super::LLMClient;
use crate::config::{ModelConfig, OpenAIProvider};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

// --- Data Structures (compatible with OpenAI/vLLM) ---
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
    // Add other parameters like top_p, etc., if needed
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

// --- Client Implementation ---
pub struct OpenAIClient {
    api_key: String,
    model_name: String,
    api_base: String,
    client: Client,
    model_config: ModelConfig,
}

impl OpenAIClient {
    pub fn new(config: &OpenAIProvider) -> Result<Self> {
        let api_key = config.api_key.clone();
        let model_name = config.default_model.clone();
        let api_base = config
            .api_base
            .as_ref()
            .unwrap_or(&"https://api.openai.com/v1".to_string())
            .clone();

        let model_config = config.models.get(&model_name)
            .or_else(|| config.models.get("default"))
            .ok_or_else(|| anyhow!("Configuration for model '{}' not found, and no default configuration available.", model_name))?
            .clone();

        let mut client_builder = Client::builder().user_agent(FAKE_USER_AGENT);

        if let Some(proxy_url) = &config.proxy {
            let proxy = reqwest::Proxy::all(proxy_url)
                .map_err(|e| anyhow!("Failed to create proxy: {}", e))?;
            client_builder = client_builder.proxy(proxy);
        }

        let client = client_builder.build()?;

        Ok(Self {
            api_key,
            model_name,
            api_base: format!("{}/chat/completions", api_base.trim_end_matches('/')),
            client,
            model_config,
        })
    }
}

#[async_trait]
impl LLMClient for OpenAIClient {
    fn model_config(&self) -> &ModelConfig {
        &self.model_config
    }

    async fn call(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
        self.call_with_retry(system_prompt, user_prompt, 3).await
    }
}

impl OpenAIClient {
    /// 带重试机制的 API 调用
    async fn call_with_retry(&self, system_prompt: &str, user_prompt: &str, max_retries: usize) -> Result<String> {
        let mut last_error = None;

        for attempt in 1..=max_retries {
            match self.make_api_call(system_prompt, user_prompt).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);

                    if attempt < max_retries {
                        let delay = Duration::from_secs(2_u64.pow(attempt as u32 - 1)); // 指数退避：1s, 2s, 4s
                        eprintln!("⚠️  LLM 调用失败 (尝试 {}/{}), {}秒后重试...", attempt, max_retries, delay.as_secs());
                        sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("所有重试都失败了")))
    }

    /// 执行单次 API 调用
    async fn make_api_call(&self, system_prompt: &str, user_prompt: &str) -> Result<String> {
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
            temperature: 0.7,
        };

        let res = self
            .client
            .post(&self.api_base)
            .bearer_auth(&self.api_key)
            .json(&request_payload)
            .timeout(Duration::from_secs(120)) // 2分钟超时
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    anyhow!("LLM API 调用超时 (120秒)")
                } else if e.is_connect() {
                    anyhow!("无法连接到 LLM API 服务器: {}", e)
                } else {
                    anyhow!("LLM API 请求失败: {}", e)
                }
            })?;

        let res_status = res.status();

        if res_status.is_success() {
            let response = res
                .json::<OpenAIResponse>()
                .await
                .map_err(|e| anyhow!("解析 LLM API 响应失败: {}", e))?;

            if let Some(first_choice) = response.choices.first() {
                let content = first_choice.message.content.trim();
                if content.is_empty() {
                    Err(anyhow!("LLM 返回了空响应"))
                } else {
                    Ok(content.to_string())
                }
            } else {
                Err(anyhow!("LLM API 响应中没有选择项"))
            }
        } else {
            let error_body = res
                .text()
                .await
                .unwrap_or_else(|_| "无法获取错误详情".to_string());

            let error_msg = match res_status.as_u16() {
                401 => "API 密钥无效或已过期",
                403 => "API 访问被拒绝",
                429 => "API 调用频率限制",
                500..=599 => "LLM 服务器内部错误",
                _ => "未知错误",
            };

            Err(anyhow!(
                "LLM API 调用失败 ({}): {}\n详细信息: {}",
                res_status,
                error_msg,
                error_body
            ))
        }
    }
}
