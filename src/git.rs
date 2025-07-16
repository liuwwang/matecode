//! src/git.rs
use crate::config::{get_config_dir, ModelConfig};
use anyhow::{anyhow, Context, Result};
use async_recursion::async_recursion;
use ignore::WalkBuilder;
use std::path::Path;
use tokio::process::Command;

/// Asynchronously runs a git command and returns the output.
pub async fn run_git_command(args: &[&str]) -> Result<std::process::Output> {
    let output = Command::new("git").args(args).output().await?;
    if !output.status.success() {
        return Err(anyhow!(
            "Git command failed: {:?}\nStderr: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(output)
}

/// Retrieves the staged git diff.
pub async fn get_staged_diff() -> Result<String> {
    let output = run_git_command(&["diff", "--staged"]).await?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Gets the project name from the git repository's root directory.
pub async fn get_project_name() -> Result<String> {
    let output = run_git_command(&["rev-parse", "--show-toplevel"]).await?;
    let project_path = Path::new(std::str::from_utf8(&output.stdout)?.trim());
    let project_name = project_path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .context("Could not get project name from path")?;
    Ok(project_name)
}

/// Gets the last commit message from git.
pub async fn get_last_commit_message() -> Result<String> {
    let output = run_git_command(&["log", "-1", "--pretty=%B"]).await?;
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// 构建忽略规则匹配器，支持项目 .gitignore 和 .matecode-ignore
async fn build_ignore_matcher() -> Result<ignore::gitignore::Gitignore> {
    let mut builder = ignore::gitignore::GitignoreBuilder::new(".");

    // 1. 添加项目根目录下的 .gitignore 文件
    let project_gitignore = std::path::Path::new(".gitignore");
    if project_gitignore.exists() {
        if let Some(e) = builder.add(project_gitignore) {
            eprintln!("警告: 无法加载项目 .gitignore 文件 {}: {}", project_gitignore.display(), e);
        }
    }

    // 2. 添加 matecode 配置目录下的 .matecode-ignore 文件
    if let Ok(config_dir) = get_config_dir().await {
        let matecode_ignore = config_dir.join(".matecode-ignore");
        if matecode_ignore.exists() {
            if let Some(e) = builder.add(&matecode_ignore) {
                eprintln!("警告: 无法加载 .matecode-ignore 文件 {}: {}", matecode_ignore.display(), e);
            }
        }
    }

    Ok(builder.build()?)
}

/// Generates a string representation of the project file tree.
async fn get_project_tree() -> Result<String> {
    let mut project_tree = String::new();
    let ignore_matcher = build_ignore_matcher().await?;

    for result in WalkBuilder::new(".").build() {
        if let Ok(entry) = result {
            if entry.path().is_dir() {
                continue;
            }
            if ignore_matcher
                .matched(entry.path(), entry.file_type().unwrap().is_dir())
                .is_ignore()
            {
                continue;
            }
            project_tree.push_str(&format!("- {}\n", entry.path().display()));
        }
    }
    Ok(project_tree)
}

/// 项目上下文信息
#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub project_tree: String,
    pub total_files: usize,
    pub affected_files: Vec<String>,
}

/// Represents a chunk of a git diff.
#[derive(Debug, Clone)]
pub struct DiffChunk {
    pub content: String,
    pub files: Vec<String>,
}

/// Represents the analysis of a git diff.
#[derive(Debug)]
pub struct DiffAnalysis {
    pub context: ProjectContext,
    pub chunks: Vec<DiffChunk>,
    pub needs_chunking: bool,
}

/*
fn estimate_token_count(text: &str) -> usize {
    text.len() / 3
}
*/

pub async fn get_affected_files() -> Result<Vec<String>> {
    let head_exists = run_git_command(&["rev-parse", "--verify", "HEAD"])
        .await
        .is_ok();
    let parent_ref = if head_exists {
        "HEAD"
    } else {
        // A magic number representing an empty tree in git
        "4b825dc642cb6eb9a060e54bf8d69288fbee4904"
    };

    let name_only_output = run_git_command(&[
        "diff-index",
        "--cached",
        "--name-only",
        "--no-renames",
        parent_ref,
    ])
    .await?;

    let files: Vec<String> = String::from_utf8_lossy(&name_only_output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect();

    Ok(files)
}

fn group_diff_by_file(diff: &str) -> Result<Vec<(String, String)>> {
    let mut files = Vec::new();
    let mut current_file_path: Option<String> = None;
    let mut current_diff = String::new();

    for line in diff.lines() {
        if line.starts_with("diff --git") {
            if let Some(path) = current_file_path.take() {
                if !current_diff.is_empty() {
                    files.push((path, current_diff.clone()));
                }
                current_diff.clear();
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(p) = parts.get(3) {
                current_file_path = Some(p.strip_prefix("b/").unwrap_or(p).to_string());
            }
        }
        current_diff.push_str(line);
        current_diff.push('\n');
    }

    if let Some(path) = current_file_path {
        if !current_diff.is_empty() {
            files.push((path, current_diff));
        }
    }

    Ok(files)
}

#[async_recursion]
async fn split_diff_by_size(diff: &str, max_chunk_size: usize) -> Result<Vec<DiffChunk>> {
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_files: Vec<String> = Vec::new();
    let mut chunk_size = 0;

    let all_files = get_affected_files().await?;

    for (file_path, file_diff) in group_diff_by_file(diff)? {
        let file_size = file_diff.len();
        if chunk_size + file_size > max_chunk_size && !current_chunk.is_empty() {
            chunks.push(DiffChunk {
                content: current_chunk.clone(),
                files: current_files.clone(),
            });
            current_chunk.clear();
            current_files.clear();
            chunk_size = 0;
        }
        current_chunk.push_str(&file_diff);
        current_files.push(file_path);
        chunk_size += file_size;
    }

    if !current_chunk.is_empty() {
        chunks.push(DiffChunk {
            content: current_chunk,
            files: current_files,
        });
    }

    if chunks.is_empty() {
        chunks.push(DiffChunk {
            content: diff.to_string(),
            files: all_files,
        });
    }

    Ok(chunks)
}

pub async fn analyze_diff(diff: &str, config: &ModelConfig) -> Result<DiffAnalysis> {
    let project_tree = get_project_tree()
        .await
        .unwrap_or_else(|_| "Could not read project tree.".to_string());

    let affected_files = get_affected_files().await?;

    let context = ProjectContext {
        project_tree: project_tree.clone(),
        total_files: affected_files.len(),
        affected_files: affected_files.clone(),
    };

    let context_size = project_tree.len() + affected_files.join(", ").len();
    let available_tokens = config.max_tokens.saturating_sub(context_size / 3);
    let max_chunk_chars = available_tokens * 3;

    let needs_chunking = diff.len() > max_chunk_chars;

    let chunks = if needs_chunking {
        split_diff_by_size(diff, max_chunk_chars).await?
    } else {
        vec![DiffChunk {
            content: diff.to_string(),
            files: affected_files,
        }]
    };

    Ok(DiffAnalysis {
        context,
        chunks,
        needs_chunking,
    })
}

/*
fn extract_file_path_from_diff_line(line: &str) -> Option<String> {
    // 解析 "diff --git a/path/to/file b/path/to/file" 格式
    if let Some(start) = line.find("b/") {
        let path_part = &line[start + 2..];
        if let Some(end) = path_part.find(' ') {
            Some(path_part[..end].to_string())
        } else {
            Some(path_part.to_string())
        }
    } else {
        None
    }
}
*/


/// 生成项目目录树
pub async fn generate_project_tree() -> Result<String> {
    let mut tree = String::new();
    tree.push_str("项目结构：\n");
    
    // 构建忽略规则匹配器
    let ignore_matcher = build_ignore_matcher().await?;
    
    // 获取项目根目录下的文件和目录
    let root_path = std::path::Path::new(".");
    generate_tree_recursive(root_path, &mut tree, "", 0, 3, &ignore_matcher).await?; // 限制深度为3
    
    Ok(tree)
}

#[async_recursion]
async fn generate_tree_recursive(
    path: &Path,
    tree: &mut String,
    prefix: &str,
    depth: u8,
    max_depth: u8,
    ignore_matcher: &ignore::gitignore::Gitignore,
) -> Result<()> {
    if depth >= max_depth {
        return Ok(());
    }
    let mut entries = vec![];
    let mut read_dir = tokio::fs::read_dir(path).await?;
    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let entry_path = entry.path();
        let is_dir = entry_path.is_dir();
        
        // 检查是否应该忽略此文件/目录
        if ignore_matcher.matched(&entry_path, is_dir).is_ignore() {
            continue;
        }
        
        entries.push(entry);
    }

    entries.sort_by_key(|a| a.path());
    let mut it = entries.iter().peekable();
    while let Some(entry) = it.next() {
        let path = entry.path();
        let next_prefix = if it.peek().is_some() {
            "│   "
        } else {
            "    "
        };
        let connector = if it.peek().is_some() { "├──" } else { "└──" };
        tree.push_str(&format!(
            "{}{} {}\n",
            prefix,
            connector,
            path.file_name().unwrap().to_str().unwrap()
        ));
        if path.is_dir() {
            generate_tree_recursive(
                &path,
                tree,
                &format!("{}{}", prefix, next_prefix),
                depth + 1,
                max_depth,
                ignore_matcher,
            )
            .await?;
        }
    }
    Ok(())
}
