//! src/hook.rs

use crate::git::run_git_command;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Finds the .git/hooks directory.
async fn get_hooks_dir() -> Result<PathBuf> {
    let git_dir_output = run_git_command(&["rev-parse", "--git-dir"]).await?;
    let git_dir = String::from_utf8(git_dir_output.stdout)?
        .trim()
        .to_string();
    Ok(PathBuf::from(git_dir).join("hooks"))
}

/// Installs the post-commit hook.
pub async fn install_post_commit_hook() -> Result<()> {
    let hooks_dir = get_hooks_dir().await?;
    if !hooks_dir.exists() {
        fs::create_dir_all(&hooks_dir).context("Failed to create hooks directory.")?;
    }
    let hook_path = hooks_dir.join("post-commit");
    let hook_script = r#"#!/bin/sh
# matecode post-commit hook

matecode archive
"#;
    fs::write(&hook_path, hook_script).context("Failed to write post-commit hook.")?;
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms).context("Failed to set hook permissions.")?;
    }
    println!("✅ post-commit hook installed successfully at {:?}", hook_path);
    Ok(())
} 