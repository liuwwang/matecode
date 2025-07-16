//! src/llm/gemini.rs
use super::LLMClient;
use crate::config::{ModelConfig, GeminiProvider};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct GeminiRequest<'a> {
    contents: Vec<Content<'a>>,
}

#[derive(Serialize)]
struct Content<'a> {
    parts: Vec<Part<'a>>,
}

#[derive(Serialize)]
struct Part<'a> {
    text: &'a str,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: Option<ContentResponse>,
}

#[derive(Deserialize, Debug, Clone)]
struct ContentResponse {
    parts: Vec<PartResponse>,
}

#[derive(Deserialize, Debug, Clone)]
struct PartResponse {
    text: Option<String>,
}

const FAKE_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

pub struct GeminiClient {
    api_key: String,
    model_name: String,
    client: Client,
    model_config: ModelConfig,
}

impl GeminiClient {
    pub fn new(config: &GeminiProvider) -> Result<Self> {
        let api_key = config.api_key.clone();
        let model_name = config.default_model.clone();

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
            client,
            model_config,
        })
    }
}

#[async_trait::async_trait]
impl LLMClient for GeminiClient {
    fn name(&self) -> &str {
        "Gemini"
    }

    fn model_config(&self) -> &ModelConfig {
        &self.model_config
    }

    async fn call(&self, _system_prompt: &str, user_prompt: &str) -> Result<String> {
        let api_url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model_name, self.api_key
        );

        let request_payload = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part { text: user_prompt }],
            }],
        };

        let res = self
            .client
            .post(&api_url)
            .json(&request_payload)
            .send()
            .await?;

        let res_status = res.status();

        if res_status.is_success() {
            let response_result = res.json::<GeminiResponse>().await;
            match response_result {
                Ok(response) => {
                    let text = response
                        .candidates
                        .first()
                        .and_then(|c| c.content.as_ref())
                        .and_then(|content| content.parts.first())
                        .and_then(|part| part.text.as_ref())
                        .map(String::from)
                        .unwrap_or_default();
                    Ok(text)
                }
                Err(e) => Err(anyhow!("无法从 Gemini API 响应中提取文本: {}", e)),
            }
        } else {
            let error_body = res.text().await?;
            Err(anyhow!(
                "调用 Gemini API 失败: {} {}\n响应体: {}",
                res_status,
                res_status.canonical_reason().unwrap_or(""),
                error_body
            ))
        }
    }
}
