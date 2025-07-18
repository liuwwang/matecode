//! src/history.rs

use crate::config::get_config_dir;
use anyhow::{Context, Result};
use chrono::NaiveDate;
use std::collections::BTreeMap;
use std::path::PathBuf;
use tokio::fs;

pub async fn get_history_dir() -> Result<PathBuf> {
    let config_dir = get_config_dir().await?;
    let history_dir = config_dir.join("history");
    if !history_dir.exists() {
        fs::create_dir_all(&history_dir)
            .await
            .context("Failed to create history directory")?;
    }
    Ok(history_dir)
}

pub async fn get_all_commits_in_range(
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<BTreeMap<String, Vec<String>>> {
    let history_dir = get_history_dir().await?;
    let mut all_projects_commits: BTreeMap<String, Vec<String>> = BTreeMap::new();

    let mut project_entries = fs::read_dir(history_dir)
        .await
        .context("Failed to read history directory")?;
    while let Some(project_entry) = project_entries.next_entry().await? {
        let project_path = project_entry.path();
        if !project_path.is_dir() {
            continue;
        }

        let project_name = project_entry.file_name().to_string_lossy().to_string();
        let mut commits_for_project: Vec<String> = Vec::new();

        if let Ok(mut day_entries) = fs::read_dir(project_path).await {
            while let Some(day_entry) = day_entries.next_entry().await? {
                let day_path = day_entry.path();
                if day_path.is_file() {
                    if let Some(filename_str) = day_path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(date) = NaiveDate::parse_from_str(filename_str, "%Y-%m-%d") {
                            if date >= start_date && date <= end_date {
                                if let Ok(content) = fs::read_to_string(&day_path).await {
                                    // Split commits by "---" and add to list
                                    for commit in content
                                        .split("\n\n---\n\n")
                                        .map(|s| s.trim())
                                        .filter(|s| !s.is_empty())
                                    {
                                        commits_for_project.push(commit.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if !commits_for_project.is_empty() {
            all_projects_commits.insert(project_name, commits_for_project);
        }
    }

    Ok(all_projects_commits)
}
