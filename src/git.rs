//! src/git.rs

use crate::config::ModelConfig;
use anyhow::{anyhow, Context, Result};
use std::process::Stdio;
use tokio::process::Command;

// --- Data Structures ---

#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub project_tree: String,
    pub total_files: usize,
    pub affected_files: Vec<String>,
}

impl ProjectContext {
    // 移除未使用的 new() 方法
}

#[derive(Debug, Clone)]
pub struct DiffChunk {
    pub files: Vec<String>,
    pub content: String,
}

impl DiffChunk {
    pub fn new(files: Vec<String>, content: String) -> Self {
        Self { files, content }
    }
}

#[derive(Debug)]
pub struct DiffAnalysis {
    pub context: ProjectContext,
    pub chunks: Vec<DiffChunk>,
    pub needs_chunking: bool,
}

// --- Public API ---

pub async fn run_git_command(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("Failed to execute git command")?;

    if output.status.success() {
        Ok(String::from_utf8(output.stdout).context("Failed to parse git command output")?)
    } else {
        let stderr = String::from_utf8(output.stderr).unwrap_or_else(|_| "Could not read stderr".to_string());
        Err(anyhow!(
            "Git command failed with status {}:\n{}",
            output.status,
            stderr
        ))
    }
}

pub async fn get_staged_diff() -> Result<String> {
    run_git_command(&["diff", "--staged"]).await
}

pub async fn get_project_name() -> Result<String> {
    let output = run_git_command(&["rev-parse", "--show-toplevel"]).await?;
    let path = std::path::Path::new(output.trim());
    Ok(path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown_project")
        .to_string())
}

pub async fn get_last_commit_message() -> Result<String> {
    run_git_command(&["log", "-1", "--pretty=%B"]).await
}

pub async fn analyze_diff(diff: &str, model_config: &ModelConfig) -> Result<DiffAnalysis> {
    let project_context = get_project_context().await?;
    let available_tokens = model_config.max_tokens - model_config.reserved_tokens;

    // 首先估算整个 diff 的 token 数
    let total_tokens = estimate_token_count(diff);

    // 如果整个 diff 在可用 token 限制内，直接返回单个块
    if total_tokens <= available_tokens {
        return Ok(DiffAnalysis {
            context: project_context.clone(),
            chunks: vec![DiffChunk::new(
                project_context.affected_files.clone(),
                diff.to_string(),
            )],
            needs_chunking: false,
        });
    }

    // 如果超过限制，需要分块。使用可用 token 的 3/4 作为每个块的硬限制，以留出余量。
    let chunking_token_limit = (available_tokens * 3) / 4;
    let chunks = chunk_large_text(diff, chunking_token_limit);
    let diff_chunks = chunks
        .into_iter()
        .map(|chunk_content| {
            DiffChunk::new(project_context.affected_files.clone(), chunk_content)
        })
        .collect();

    Ok(DiffAnalysis {
        context: project_context,
        chunks: diff_chunks,
        needs_chunking: true,
    })
}

// --- Helper Functions ---

async fn get_project_context() -> Result<ProjectContext> {
    let affected_files_str = run_git_command(&["diff", "--staged", "--name-only"]).await?;
    let affected_files = affected_files_str.lines().map(String::from).collect();

    // For simplicity, we're not generating the full file tree for now to keep it fast.
    // This could be an enhancement for later.
    let project_tree = "File tree generation is disabled for performance.".to_string();
    let total_files = 0; // Not currently implemented.

    Ok(ProjectContext {
        project_tree,
        total_files,
        affected_files,
    })
}

fn estimate_token_count(text: &str) -> usize {
    // A simple heuristic: 1 token is roughly 3-4 characters.
    // Using a ratio of 3 for a more conservative (safer) estimate.
    (text.len() as f64 / 3.0).ceil() as usize
}

fn chunk_large_text(text: &str, token_limit: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_tokens = 0;

    for line in text.lines() {
        let line_tokens = estimate_token_count(line);

        if current_tokens + line_tokens > token_limit && !current_chunk.is_empty() {
            chunks.push(current_chunk.clone());
            current_chunk.clear();
            current_tokens = 0;
        }

        current_chunk.push_str(line);
        current_chunk.push('\n');
        current_tokens += line_tokens;
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    chunks
}
