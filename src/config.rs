//! src/config.rs

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::llm::gemini::GeminiClient;
use crate::llm::openai::OpenAIClient;
use crate::llm::LLM;

/// 读取LLM_PROVIDER环境变量以确定使用哪个客户端。
fn get_provider_name() -> String {
    env::var("LLM_PROVIDER").unwrap_or_else(|_| "gemini".to_string())
}

/// Factory功能，根据配置获取LLM客户端。
pub fn get_llm_client() -> Result<LLM<'static>> {
    let config = load_config()?;
    crate::llm::create_llm_client(config.llm)
}

/// Returns the configuration directory path (~/.config/matecode).
pub async fn get_config_dir() -> Result<PathBuf> {
    let config_dir = if cfg!(windows) {
        // Windows: %APPDATA%\matecode
        dirs::data_dir()
            .map(|p| p.join("matecode"))
            .context("Could not get data directory")?
    } else {
        // Linux/macOS: ~/.config/matecode
        dirs::config_dir()
            .map(|p| p.join("matecode"))
            .context("Could not get config directory")?
    };

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .await
            .context("Could not create config directory")?;
    }
    Ok(config_dir)
}

/// Represents the main configuration for the application.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// LLM provider settings.
    pub llm: LLMConfig,
    /// Configuration for context length and token limits.
    pub context: ContextConfig,
    /// The default LLM provider.
    pub default_llm: String,
}

/// Defines the context window configuration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContextConfig {
    /// The maximum number of tokens to use for the context.
    pub max_tokens: usize,
}

/// Defines the LLM provider and model to use.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LLMConfig {
    /// The name of the LLM provider (e.g., "openai", "gemini").
    pub provider: String,
    /// The specific model name to use (e.g., "gpt-4", "gemini-pro").
    pub model: Option<String>,
}

/// Creates a default configuration file if one does not exist.
pub async fn create_default_config() -> Result<()> {
    let config_dir = get_config_dir().await?;
    let config_path = config_dir.join("config.toml");
    let default_config = Config {
        default_llm: "openai".to_string(),
        llm: LLMConfig {
            provider: "openai".to_string(),
            model: Some("gpt-4".to_string()),
        },
        context: ContextConfig {
            max_tokens: 4096,
        },
    };

    let config_content = toml::to_string(&default_config)?;
    let mut file = fs::File::create(&config_path).await?;
    file.write_all(config_content.as_bytes()).await?;

    println!("Created default config file at {:?}", config_path);
    Ok(())
}

pub async fn load_config() -> Result<Config> {
    let config_dir = get_config_dir().await?;
    let config_path = config_dir.join("config.toml");

    if !config_path.exists() {
        create_default_config().await?;
    }

    let config_content = fs::read_to_string(config_path)
        .await
        .context("Could not read config file")?;
    let config: Config =
        toml::from_str(&config_content).context("Could not parse config file")?;

    Ok(config)
}
