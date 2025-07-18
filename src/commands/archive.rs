use crate::git;
use crate::history::get_history_dir;
use anyhow::{Context, Result};
use tokio::fs;

pub async fn archive_commit_message(project_name: &str, message: &str) -> Result<()> {
    let project_history_dir = get_history_dir().await?.join(project_name);
    if !project_history_dir.exists() {
        fs::create_dir_all(&project_history_dir)
            .await
            .context("Failed to create project history directory")?;
    }

    let date_str = chrono::Local::now().format("%Y-%m-%d").to_string();
    let file_path = project_history_dir.join(format!("{date_str}.md"));

    let mut existing_content = if file_path.exists() {
        fs::read_to_string(&file_path)
            .await
            .context("Failed to read existing history file")?
    } else {
        String::new()
    };

    if !existing_content.is_empty() {
        existing_content.push_str("\n\n---\n\n");
    }
    existing_content.push_str(message);

    fs::write(file_path, existing_content)
        .await
        .context("Failed to write to history file")?;

    Ok(())
}

pub async fn handle_archive() -> Result<()> {
    let project_name = git::get_git_repo_name()
        .await
        .context("无法获取用于归档的项目名称。")?;
    let commit_message = git::get_last_commit_message()
        .await
        .context("无法获取用于归档的最后一条提交信息。")?;
    archive_commit_message(&project_name, &commit_message)
        .await
        .context("无法归档提交信息。")?;
    Ok(())
}
