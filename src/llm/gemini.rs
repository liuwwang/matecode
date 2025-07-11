//! src/llm/gemini.rs
use super::LLMClient;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

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

#[derive(Deserialize, Debug)]
struct ContentResponse {
    parts: Vec<PartResponse>,
}

#[derive(Deserialize, Debug)]
struct PartResponse {
    text: Option<String>,
}

pub struct GeminiClient {
    api_key: String,
    model_name: String,
}

impl GeminiClient {
    pub fn new() -> Result<Self> {
        let api_key = env::var("GEMINI_API_KEY")
            .map_err(|_| anyhow!("错误：请在 .env 文件中设置 GEMINI_API_KEY"))?;
        let model_name =
            env::var("GEMINI_MODEL_NAME").unwrap_or_else(|_| "gemini-1.5-pro-latest".to_string());
        Ok(Self {
            api_key,
            model_name,
        })
    }
}

#[async_trait::async_trait]
impl LLMClient for GeminiClient {
    fn name(&self) -> &str {
        "Gemini"
    }

    async fn call(&self, user_prompt: &str) -> Result<String> {
        let client = Client::new();
        let api_url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model_name, self.api_key
        );

        let request_payload = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part { text: user_prompt }],
            }],
        };

        let res = client.post(&api_url).json(&request_payload).send().await?;

        let res_status = res.status();

        if res_status.is_success() {
            let response_result = res.json::<GeminiResponse>().await;
            match response_result {
                Ok(response) => {
                    let text = response
                        .candidates
                        .first()
                        .and_then(|c| c.content.clone())
                        .and_then(|c| c.parts.first())
                        .and_then(|p| p.text.clone())
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
