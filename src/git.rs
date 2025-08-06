use crate::config;
use anyhow::{Context, Result, anyhow};
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub project_tree: String,
    pub total_files: usize,
    pub affected_files: Vec<String>,
}

impl ProjectContext {}

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

/// 运行git命令
pub async fn run_git_command(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .context("执行Git command 失败")?;

    if output.status.success() {
        Ok(String::from_utf8(output.stdout).context("git命令解析成功")?)
    } else {
        let stderr = String::from_utf8(output.stderr)
            .unwrap_or_else(|_| "Could not read stderr".to_string());

        Err(anyhow!(
            "Git command 执行失败, status: {}\n{}",
            output.status,
            stderr
        ))
    }
}

/// 获取暂存区的diff信息
pub async fn get_staged_diff() -> Result<String> {
    run_git_command(&["diff", "--staged"]).await
}

/// 获取git项目名称
pub async fn get_git_repo_name() -> Result<String> {
    let output = run_git_command(&["rev-parse", "--show-toplevel"]).await?;

    let path = std::path::Path::new(output.trim());

    Ok(path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown_project")
        .to_string())
}

/// 获取最近一条commit-message
pub async fn get_last_commit_message() -> Result<String> {
    run_git_command(&["log", "-1", "--pretty=%B"]).await
}

/// 判断当前目录是否是一个git仓库
pub async fn check_is_git_repo() -> bool {
    run_git_command(&["rev-parse", "--is-inside-work-tree"])
        .await
        .map(|s| s.trim() == "true")
        .unwrap_or(false)
}

/// 获取暂存区文件列表
pub async fn get_staged_files() -> Result<Vec<String>> {
    let output = run_git_command(&["diff", "--name-only", "--cached"]).await?;
    Ok(output.lines().map(String::from).collect())
}

/// 获取项目上下文信息
pub async fn get_project_context() -> Result<ProjectContext> {
    let affected_files_str = run_git_command(&["diff", "--staged", "--name-only"]).await?;
    let affected_files = affected_files_str.lines().map(String::from).collect();

    let project_tree = "File tree generation is disabled for performance.".to_string();

    let total_files = 0;

    Ok(ProjectContext {
        project_tree,
        total_files,
        affected_files,
    })
}

pub fn estimeate_token_count(text: &str) -> usize {
    (text.len() as f64 / 3.0).ceil() as usize
}

pub fn chunk_large_text(text: &str, token_limit: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_tokens = 0;

    for line in text.lines() {
        let line_tokens = estimeate_token_count(line);
        if current_tokens + line_tokens > token_limit && !current_chunk.is_empty() {
            chunks.push(current_chunk.clone());
            current_chunk.clear();
            current_tokens = 0;
        }

        current_chunk.push_str(line);
        current_chunk.push("\n".parse().unwrap());
        current_tokens += line_tokens;
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }
    chunks
}

/// diff内容分析，主要分析内容长度,进行合适的分割处理
pub async fn analyze_diff(diff: &str, model_config: &config::ModelConfig) -> Result<DiffAnalysis> {
    // 项目上下文
    let project_context = get_project_context().await?;

    // 剩余可用tokens
    let available_tokens = model_config.max_tokens - model_config.reserved_tokens;

    // 估算的token，以后可以使用标准的分词器进行计算
    let total_tokens = estimeate_token_count(diff);

    // 可以直接使用一个提交处理
    if total_tokens <= available_tokens {
        return Ok(DiffAnalysis {
            context: project_context.clone(),
            chunks: vec![DiffChunk::new(
                project_context.affected_files.clone(),
                diff.to_string(),
            )],
            needs_chunking: false,
        });
    } else {
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
}
