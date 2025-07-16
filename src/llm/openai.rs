//! src/llm/openai.rs
use super::LLMClient;
use crate::config::{ModelConfig, OpenAIProvider};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

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
    model_config: ModelConfig,
}

impl OpenClient {
    pub fn new(config: &OpenAIProvider) -> Result<Self> {
        let api_key = config.api_key.clone();
        let model_name = config.default_model.clone();
        let api_base = config.api_base.as_ref()
            .unwrap_or(&"https://api.openai.com/v1".to_string())
            .clone();

        // 获取模型配置，如果找不到具体模型配置，使用 default 配置
        let model_config = config.models.get(&model_name)
            .or_else(|| config.models.get("default"))
            .ok_or_else(|| anyhow!("未找到模型 {} 的配置，也没有找到默认配置", model_name))?
            .clone();

        // 构建 HTTP 客户端
        let mut client_builder = Client::builder().user_agent(FAKE_USER_AGENT);
        
        // 如果配置了代理，使用代理
        if let Some(proxy_url) = &config.proxy {
            let proxy = reqwest::Proxy::all(proxy_url)?;
            client_builder = client_builder.proxy(proxy);
        }
        
        let client = client_builder.build()?;

        Ok(Self {
            api_key,
            model_name,
            api_base: format!("{}/chat/completions", api_base),
            client,
            model_config,
        })
    }
}

#[async_trait::async_trait]
impl LLMClient for OpenClient {
    fn name(&self) -> &str {
        "OpenAI"
    }

    fn model_config(&self) -> &ModelConfig {
        &self.model_config
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
