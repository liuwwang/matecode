//! src/git.rs
use crate::config::get_config_dir;
use anyhow::Result;
use ignore::gitignore::GitignoreBuilder;
use std::path::Path;
use std::process::{Command, Output};

pub fn get_project_name() -> Result<String> {
    let output = run_git_command(&["rev-parse", "--show-toplevel"])?;
    let repo_path_str = String::from_utf8(output.stdout)?.trim().to_string();
    let repo_path = Path::new(&repo_path_str);
    let project_name = repo_path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("无法获取项目名称"))?;
    Ok(project_name)
}

pub fn get_last_commit_message() -> Result<String> {
    let output = run_git_command(&["log", "-1", "--pretty=%B"])?;
    let message = String::from_utf8(output.stdout)?.trim().to_string();
    if message.is_empty() {
        Err(anyhow::anyhow!("没有找到上一次的提交信息"))
    } else {
        Ok(message)
    }
}

/// 项目上下文信息
#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub project_tree: String,
    pub affected_files: Vec<String>,
    pub total_files: usize,
}

/// diff分块信息
#[derive(Debug, Clone)]
pub struct DiffChunk {
    pub content: String,
    pub files: Vec<String>,
    pub size: usize,
}

/// 分析结果
#[derive(Debug)]
pub struct DiffAnalysis {
    pub context: ProjectContext,
    pub chunks: Vec<DiffChunk>,
    pub total_size: usize,
    pub needs_chunking: bool,
}

// 估算的字符到token的转换比例（粗略估计：1 token ≈ 3-4 个字符）
const CHARS_PER_TOKEN: usize = 3;

pub fn run_git_command(args: &[&str]) -> Result<Output> {
    // 跨平台的 Git 命令调用
    let git_cmd = if cfg!(windows) {
        // Windows 上优先尝试 git.exe
        "git.exe"
    } else {
        "git"
    };

    let output = Command::new(git_cmd).args(args).output().or_else(|_| {
        // 如果失败，尝试另一种方式
        let fallback_cmd = if cfg!(windows) { "git" } else { "git.exe" };
        Command::new(fallback_cmd).args(args).output()
    })?;

    if output.status.success() {
        Ok(output)
    } else {
        Err(anyhow::anyhow!(
            "Git 命令执行失败 (退出码: {}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// 获取暂存区（staged）的代码变更，并应用 .gitignore 和 .matecode-ignore 的规则
pub fn get_staged_diff() -> Result<String> {
    // Determine the reference to compare against.
    // If HEAD exists, use it. Otherwise, assume initial commit and use the empty tree.
    let head_exists = run_git_command(&["rev-parse", "--verify", "HEAD"]).is_ok();
    let parent_ref = if head_exists {
        "HEAD"
    } else {
        // This is the magic number for an empty tree in Git, used for the initial commit.
        "4b825dc642cb6eb9a060e54bf8d69288fbee4904"
    };

    // 1. 获取所有暂存的文件名
    let name_only_output = run_git_command(&[
        "diff-index",
        "--cached",
        "--name-only",
        "--no-renames",
        parent_ref,
    ])?;
    let staged_files_raw = String::from_utf8_lossy(&name_only_output.stdout);
    if staged_files_raw.trim().is_empty() {
        return Ok(String::new());
    }

    // 2. 构建一个 Gitignore 匹配器
    let mut builder = GitignoreBuilder::new(".");

    // 添加用户自定义的忽略文件（从家目录的配置文件夹读取）
    if let Ok(config_dir) = get_config_dir() {
        let matecode_ignore_path = config_dir.join(".matecode-ignore");
        if matecode_ignore_path.exists() {
            builder.add(matecode_ignore_path);
        }
    }

    // 添加硬编码的、内置的忽略规则
    // todo: 后面可以考虑将这部分也做成可配置的
    builder.add_line(None, "*.json")?;

    let ignorer = builder.build()?;

    // 3. 在内存中筛选需要 diff 的文件
    let mut files_to_diff = Vec::new();
    for file_path_str in staged_files_raw.lines() {
        let file_path = Path::new(file_path_str);

        let is_hidden = file_path
            .components()
            .any(|c| c.as_os_str().to_string_lossy().starts_with('.'));
        if is_hidden {
            println!("🚫 已忽略隐藏文件/目录: {file_path_str}");
            continue;
        }

        let is_ignored = ignorer.matched(file_path, false).is_ignore();

        if !is_ignored {
            files_to_diff.push(file_path_str);
        } else {
            println!("🚫 已根据 .gitignore/.matecode-ignore 忽略文件: {file_path_str}");
        }
    }

    // 4. 如果没有剩下任何文件，则返回空字符串
    if files_to_diff.is_empty() {
        return Ok(String::new());
    }

    // 5. 一次性调用 git diff 获取所有未被忽略的文件的变更
    let mut command_args = vec![
        "diff-index",
        "--patch",
        "--cached",
        "--no-renames",
        parent_ref,
        "--",
    ];
    command_args.extend(files_to_diff);

    let diff_output = run_git_command(&command_args)?;
    let diff_string = String::from_utf8_lossy(&diff_output.stdout).to_string();

    Ok(diff_string)
}

/// 生成项目目录树
pub fn generate_project_tree() -> Result<String> {
    let mut tree = String::new();
    tree.push_str("项目结构：\n");
    
    // 获取项目根目录下的文件和目录
    let root_path = Path::new(".");
    generate_tree_recursive(root_path, &mut tree, "", 0, 3)?; // 限制深度为3
    
    Ok(tree)
}

fn generate_tree_recursive(
    path: &Path,
    tree: &mut String,
    prefix: &str,
    depth: usize,
    max_depth: usize,
) -> Result<()> {
    if depth > max_depth {
        return Ok(());
    }

    // 构建gitignore匹配器
    let mut builder = GitignoreBuilder::new(".");
    if let Ok(config_dir) = get_config_dir() {
        let matecode_ignore_path = config_dir.join(".matecode-ignore");
        if matecode_ignore_path.exists() {
            builder.add(matecode_ignore_path);
        }
    }
    let ignorer = builder.build()?;

    let mut entries: Vec<_> = std::fs::read_dir(path)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_string_lossy();
            
            // 过滤掉一些不必要的文件和目录
            if file_name.starts_with('.') && file_name != ".gitignore" {
                return false;
            }
            if file_name == "target" || file_name == "node_modules" {
                return false;
            }
            
            // 检查是否被gitignore忽略
            !ignorer.matched(&path, path.is_dir()).is_ignore()
        })
        .collect();

    entries.sort_by(|a, b| {
        // 目录优先，然后按名称排序
        match (a.path().is_dir(), b.path().is_dir()) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == entries.len() - 1;
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_string_lossy();
        
        let current_prefix = if is_last { "└── " } else { "├── " };
        tree.push_str(&format!("{}{}{}\n", prefix, current_prefix, file_name));
        
        if path.is_dir() && depth < max_depth {
            let next_prefix = if is_last { "    " } else { "│   " };
            generate_tree_recursive(
                &path,
                tree,
                &format!("{}{}", prefix, next_prefix),
                depth + 1,
                max_depth,
            )?;
        }
    }

    Ok(())
}

/// 获取本次修改影响的文件列表
pub fn get_affected_files() -> Result<Vec<String>> {
    let head_exists = run_git_command(&["rev-parse", "--verify", "HEAD"]).is_ok();
    let parent_ref = if head_exists {
        "HEAD"
    } else {
        "4b825dc642cb6eb9a060e54bf8d69288fbee4904"
    };

    let name_only_output = run_git_command(&[
        "diff-index",
        "--cached",
        "--name-only",
        "--no-renames",
        parent_ref,
    ])?;
    
    let files: Vec<String> = String::from_utf8_lossy(&name_only_output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect();

    Ok(files)
}

/// 分析diff内容并进行分块处理
pub fn analyze_diff(diff: &str, context_config: &crate::llm::ContextConfig) -> Result<DiffAnalysis> {
    let project_tree = generate_project_tree()?;
    let affected_files = get_affected_files()?;
    let total_files = affected_files.len();
    
    let context = ProjectContext {
        project_tree: project_tree.clone(),
        affected_files: affected_files.clone(),
        total_files,
    };

    let total_size = diff.len();
    
    // 计算项目上下文所需的token数
    let context_size = estimate_token_count(&project_tree) + 
                      estimate_token_count(&affected_files.join(", "));
    
    // 计算每个chunk的最大允许大小
    let available_tokens = context_config.available_context_tokens();
    let max_chunk_tokens = available_tokens.saturating_sub(context_size);
    let max_chunk_chars = max_chunk_tokens * CHARS_PER_TOKEN;
    
    let needs_chunking = total_size > max_chunk_chars;

    let chunks = if needs_chunking {
        split_diff_by_size(diff, max_chunk_chars)?
    } else {
        vec![DiffChunk {
            content: diff.to_string(),
            files: affected_files,
            size: total_size,
        }]
    };

    Ok(DiffAnalysis {
        context,
        chunks,
        total_size,
        needs_chunking,
    })
}

/// 估算文本的token数量
fn estimate_token_count(text: &str) -> usize {
    text.len() / CHARS_PER_TOKEN
}

/// 按大小将diff内容分块，并格式化输出
fn split_diff_by_size(diff: &str, max_chunk_size: usize) -> Result<Vec<DiffChunk>> {
    let mut chunks = Vec::new();
    let lines: Vec<&str> = diff.lines().collect();
    
    let mut current_chunk = String::new();
    let mut current_files = Vec::new();
    let mut i = 0;
    
    // 添加chunk头部
    current_chunk.push_str("=== 代码变更详情 ===\n\n");
    
    while i < lines.len() {
        let line = lines[i];
        
        // 检查是否是新文件的开始
        if line.starts_with("diff --git") {
            // 提取文件名
            if let Some(file_path) = extract_file_path_from_diff_line(line) {
                if !current_files.contains(&file_path) {
                    current_files.push(file_path.clone());
                }
                
                // 添加格式化的文件分隔符
                let file_header = format!("\n📁 文件: {}\n{}\n", file_path, "=".repeat(50));
                
                // 检查是否会超过限制
                if current_chunk.len() + file_header.len() > max_chunk_size && !current_chunk.is_empty() {
                    let chunk_size = current_chunk.len();
                    chunks.push(DiffChunk {
                        content: current_chunk.clone(),
                        files: current_files.clone(),
                        size: chunk_size,
                    });
                    current_chunk.clear();
                    current_files.clear();
                    current_chunk.push_str("=== 代码变更详情 ===\n\n");
                }
                
                current_chunk.push_str(&file_header);
            }
        }
        
        let line_with_newline = format!("{}\n", line);
        
        // 如果添加这一行会超过大小限制，并且当前chunk不为空，则创建一个新chunk
        if current_chunk.len() + line_with_newline.len() > max_chunk_size && !current_chunk.is_empty() {
            let chunk_size = current_chunk.len();
            chunks.push(DiffChunk {
                content: current_chunk.clone(),
                files: current_files.clone(),
                size: chunk_size,
            });
            current_chunk.clear();
            current_files.clear();
            current_chunk.push_str("=== 代码变更详情 ===\n\n");
        }
        
        current_chunk.push_str(&line_with_newline);
        i += 1;
    }
    
    // 添加最后一个chunk
    if !current_chunk.is_empty() {
        let chunk_size = current_chunk.len();
        chunks.push(DiffChunk {
            content: current_chunk,
            files: current_files,
            size: chunk_size,
        });
    }
    
    // 如果没有产生任何chunk，创建一个包含所有内容的chunk
    if chunks.is_empty() {
        let formatted_diff = format!("=== 代码变更详情 ===\n\n{}", diff);
        chunks.push(DiffChunk {
            content: formatted_diff,
            files: get_affected_files().unwrap_or_default(),
            size: diff.len(),
        });
    }
    
    Ok(chunks)
}

/// 从diff行中提取文件路径
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
