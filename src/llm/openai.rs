//! src/llm/openai.rs
use super::LLMClient;
use crate::config::{ModelConfig, OpenAIProvider};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

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
        let api_base = config.api_base.as_ref()
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
            temperature: 0.7, // A slightly higher temperature might yield more creative results
        };

        let res = self
            .client
            .post(&self.api_base)
            .bearer_auth(&self.api_key)
            .json(&request_payload)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send request to OpenAI API: {}", e))?;

        let res_status = res.status();

        if res_status.is_success() {
            let response = res.json::<OpenAIResponse>().await
                .map_err(|e| anyhow!("Failed to parse JSON response from OpenAI API: {}", e))?;
                
            if let Some(first_choice) = response.choices.first() {
                Ok(first_choice.message.content.trim().to_string())
            } else {
                Err(anyhow::anyhow!("API call successful, but the 'choices' array was empty."))
            }
        } else {
            let error_body = res.text().await
                .unwrap_or_else(|_| "Could not retrieve error body".to_string());
            Err(anyhow!(
                "OpenAI compatible API call failed: {}\nResponse body: {}",
                res_status,
                error_body
            ))
        }
    }
}
