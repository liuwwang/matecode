//! src/hook.rs

use crate::git::run_git_command;
use anyhow::{Context, Result};
use tokio::fs;
use std::path::PathBuf;

const HOOK_CONTENT: &str = r#"#!/bin/bash
# Post-commit hook for matecode
# This hook archives the commit message for later use in reports

# Get the project name and last commit message
PROJECT_NAME=$(basename "$(git rev-parse --show-toplevel)")
COMMIT_MESSAGE=$(git log -1 --pretty=%B)

# Archive the commit using matecode
matecode archive
"#;

pub async fn install_post_commit_hook() -> Result<()> {
    let git_dir_output = run_git_command(&["rev-parse", "--git-dir"]).await?;
    let git_dir = git_dir_output.trim().to_string();
    
    let git_dir_path = PathBuf::from(&git_dir);
    let hooks_dir = git_dir_path.join("hooks");
    
    if !hooks_dir.exists() {
        fs::create_dir_all(&hooks_dir).await.context("Failed to create hooks directory")?;
    }
    
    let hook_path = hooks_dir.join("post-commit");
    let hook_script = HOOK_CONTENT.replace("\r\n", "\n");
    
    fs::write(&hook_path, hook_script).await.context("Failed to write post-commit hook")?;
    
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&hook_path).await?.permissions();
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms).await.context("Failed to set hook permissions")?;
    }
    
    println!("✅ Post-commit hook installed successfully at: {}", hook_path.display());
    Ok(())
} 