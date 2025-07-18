//! src/hook.rs

use crate::git::run_git_command;
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, PartialEq)]
pub enum HookStatus {
    NotInstalled,
    InstalledByUs,
    InstalledByOther,
}

const HOOK_CONTENT: &str = r#"#!/bin/bash
# Post-commit hook for matecode
# This hook archives the commit message for later use in reports

# Get the project name and last commit message
PROJECT_NAME=$(basename "$(git rev-parse --show-toplevel)")
COMMIT_MESSAGE=$(git log -1 --pretty=%B)

# Archive the commit using matecode
matecode archive
"#;

async fn get_hook_path() -> Result<PathBuf> {
    let git_dir_output = run_git_command(&["rev-parse", "--git-dir"]).await?;
    let git_dir = git_dir_output.trim();
    let git_dir_path = PathBuf::from(git_dir);
    Ok(git_dir_path.join("hooks").join("post-commit"))
}

pub async fn check_hook_status() -> Result<HookStatus> {
    let hook_path = get_hook_path().await?;
    if !hook_path.exists() {
        return Ok(HookStatus::NotInstalled);
    }

    let content = fs::read_to_string(&hook_path).await?;

    // 检查是否包含 matecode archive 命令
    if content.contains("matecode archive") {
        Ok(HookStatus::InstalledByUs)
    } else {
        Ok(HookStatus::InstalledByOther)
    }
}

pub async fn install_post_commit_hook() -> Result<()> {
    let hook_path = get_hook_path().await?;
    let hooks_dir = hook_path
        .parent()
        .context("Failed to get hooks directory from path")?;

    if !hooks_dir.exists() {
        fs::create_dir_all(hooks_dir)
            .await
            .context("Failed to create hooks directory")?;
    }

    // 统一的检查和安装逻辑
    if hook_path.exists() {
        let existing_content = fs::read_to_string(&hook_path).await?;

        // 检查是否已经包含 matecode archive 命令
        if existing_content.contains("matecode archive") {
            println!("✅ Post-commit 钩子已包含 matecode archive 命令。");
            return Ok(());
        }

        // 追加命令到现有钩子
        let mut new_content = existing_content;
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str("\n# Added by matecode\nmatecode archive\n");
        fs::write(&hook_path, new_content)
            .await
            .context("Failed to append to post-commit hook")?;
        println!("✅ 已将 matecode archive 命令添加到现有的 post-commit 钩子中。");
        return Ok(());
    }

    // 创建新的钩子文件
    let hook_script = HOOK_CONTENT.replace("\r\n", "\n");
    fs::write(&hook_path, hook_script)
        .await
        .context("Failed to write post-commit hook")?;

    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&hook_path).await?.permissions();
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)
            .await
            .context("Failed to set hook permissions")?;
    }

    println!("✅ Post-commit 钩子安装成功，位置: {}", hook_path.display());
    Ok(())
}
