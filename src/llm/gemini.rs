//! src/llm/gemini.rs
use super::LLMClient;
use crate::config::{GeminiProvider, ModelConfig};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
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
            client,
            model_config,
        })
    }
}

#[async_trait]
impl LLMClient for GeminiClient {
    fn model_config(&self) -> &ModelConfig {
        &self.model_config
    }

    async fn call(&self, _system_prompt: &str, user_prompt: &str) -> Result<String> {
        // Gemini API does not have a separate system prompt, so we prepend it to the user prompt.
        let full_prompt = if !_system_prompt.is_empty() {
            format!("{_system_prompt}\n\n{user_prompt}")
        } else {
            user_prompt.to_string()
        };

        let api_url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model_name, self.api_key
        );

        let request_payload = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part { text: &full_prompt }],
            }],
        };

        let res = self
            .client
            .post(&api_url)
            .json(&request_payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send request to Gemini API: {}", e))?;

        let res_status = res.status();

        if res_status.is_success() {
            let response = res
                .json::<GeminiResponse>()
                .await
                .map_err(|e| anyhow!("Failed to parse JSON response from Gemini API: {}", e))?;

            response
                .candidates
                .first()
                .and_then(|c| c.content.as_ref())
                .and_then(|content| content.parts.first())
                .and_then(|part| part.text.as_ref())
                .map(|s| s.trim().to_string())
                .ok_or_else(|| anyhow!("Could not extract text from Gemini API response."))
        } else {
            let error_body = res
                .text()
                .await
                .unwrap_or_else(|_| "Could not retrieve error body".to_string());
            Err(anyhow!(
                "Gemini API call failed: {} {}\nResponse body: {}",
                res_status,
                res_status.canonical_reason().unwrap_or(""),
                error_body
            ))
        }
    }
}
