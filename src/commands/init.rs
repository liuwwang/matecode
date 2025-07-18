use crate::config;
use anyhow::{Context, Result};

pub async fn handle_init() -> Result<()> {
    config::create_default_config()
        .await
        .context("无法初始化配置。")?;
    Ok(())
}
