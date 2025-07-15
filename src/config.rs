//! src/config.rs

use crate::llm::{GeminiClient, OpenClient, LLM};
use anyhow::{anyhow, Result};
use std::env;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// 读取LLM_PROVIDER环境变量以确定使用哪个客户端。
fn get_provider_name() -> String {
    env::var("LLM_PROVIDER").unwrap_or_else(|_| "gemini".to_string())
}

/// Factory功能，根据配置获取LLM客户端。
pub fn get_llm_client() -> Result<LLM> {
    let provider = get_provider_name();
    match provider.as_str() {
        "gemini" => Ok(LLM::Gemini(GeminiClient::new()?)),
        "openai" => Ok(LLM::OpenAI(OpenClient::new()?)),
        "ollama" => Ok(LLM::OpenAI(OpenClient::new()?)),
        _ => Err(anyhow!("不支持的 LLM_PROVIDER: {}", provider)),
    }
}

/// 获取遵循平台约定的matecode配置目录的路径。
///
/// - Windows: %APPDATA%\matecode
/// - macOS: ~/Library/Application Support/matecode  
/// - Linux: ~/.config/matecode
pub fn get_config_dir() -> Result<PathBuf> {
    let config_dir = if cfg!(windows) {
        // Windows: %APPDATA%\matecode
        dirs::config_dir()
            .ok_or_else(|| anyhow!("无法找到配置目录"))?
            .join("matecode")
    } else if cfg!(target_os = "macos") {
        // macOS: ~/Library/Application Support/matecode
        dirs::config_dir()
            .ok_or_else(|| anyhow!("无法找到配置目录"))?
            .join("matecode")
    } else {
        // Linux: ~/.config/matecode
        dirs::config_dir()
            .ok_or_else(|| anyhow!("无法找到配置目录"))?
            .join("matecode")
    };

    Ok(config_dir)
}

/// 在`~/ `中创建默认的.env和. matcode -ignore文件。
pub async fn create_default_config() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).await?;
    }

    // Create .env file
    let env_path = config_dir.join(".env");
    if !env_path.exists() {
        let mut file = fs::File::create(&env_path).await?;
        let content = b"# --- LLM Provider Configuration ---\nLLM_PROVIDER=\"gemini\" # \"openai\" or \"ollama\"\n\n# --- Gemini Configuration ---\nGEMINI_API_KEY=\"your_gemini_api_key_here\"\nGEMINI_MODEL_NAME=\"gemini-1.5-pro-latest\"\n\n# --- OpenAI/vLLM (Compatible) Configuration ---\n#OPENAI_API_KEY=\"your_openai_api_key_here\"\n#OPENAI_API_URL=\"https://api.openai.com/v1/chat/completions\"\n#OPENAI_MODEL_NAME=\"gpt-4-turbo\"\n\n# --- Ollama (Local) Configuration ---\n#OPENAI_API_KEY=\"ollama\" # The key can be any non-empty string\n#OPENAI_API_URL=\"http://localhost:11434/v1/chat/completions\"\n#OPENAI_MODEL_NAME=\"llama3\" # Replace with your desired local model\n";
        file.write_all(content).await?;
    }

    // Create .matecode-ignore
    let ignore_path = config_dir.join(".matecode-ignore");
    if !ignore_path.exists() {
        let mut file = fs::File::create(&ignore_path).await?;
        file.write_all(b"*.lock\n").await?;
        file.write_all(b"*.log\n").await?;
    }

    Ok(config_dir)
}
