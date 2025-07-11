//! src/llm/gemini.rs
use super::LLMClient; // 从父模块导入 trait
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

// --- 数据结构定义 ---
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
    candidates: Vec<Candidates>,
}
#[derive(Deserialize, Debug)]
struct Candidates {
    content: Option<ContentResponse>,
}
#[derive(Deserialize, Debug)]
struct ContentResponse {
    parts: Vec<PartResponse>,
}
#[derive(Deserialize, Debug)]
struct PartResponse {
    text: String,
}

// --- 客户端实现 ---
pub struct GeminiClient {
    api_key: String,
    model_name: String,
}

impl GeminiClient {
    pub fn new() -> Result<Self> {
        let api_key = env::var("GEMINI_API_KEY")
            .map_err(|_| anyhow!("ERROR: Please set the GEMINI_API_KEY in your .env file"))?;
        let model_name = env::var("GEMINI_MODEL_NAME")
            .unwrap_or_else(|_| "gemini-1.5-pro-latest".to_string());
        Ok(Self { api_key, model_name })
    }
}

#[async_trait::async_trait]
impl LLMClient for GeminiClient {
    fn name(&self) -> &str {
        "Gemini"
    }

    async fn call(&self, user_prompt: &str) -> Result<String> {
        let api_url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model_name, self.api_key
        );
        let client = Client::new();
        let request_payload = GeminiRequest {
            contents: vec![Content { parts: vec![Part { text: user_prompt }] }],
        };
        let res = client.post(&api_url).json(&request_payload).send().await?;

        let res_status = res.status();

        if res_status.is_success() {
            let response = res.json::<GeminiResponse>().await?;
            let text = response
                .candidates
                .get(0)
                .and_then(|c| c.content.as_ref())
                .and_then(|c| c.parts.get(0))
                .map(|p| p.text.clone())
                .ok_or_else(|| anyhow!("Could not extract text from Gemini API response"))?;
            Ok(text.trim().to_string())
        } else {
            let error_body = res.text().await?;
            Err(anyhow!(
                "Failed to call Gemini API: {} {}\nResponse body: {}",
                res_status,
                res_status.canonical_reason().unwrap_or(""),
                error_body
            ))
        }
    }
} 