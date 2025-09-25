//! src/commands/understand.rs

use crate::config;
use crate::git;
use crate::llm::{parse_prompt_template, LLMClient};
use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use termimad::MadSkin;
use std::collections::HashMap;

/// Handles the project understanding process.
pub async fn handle_understand(dir: Option<String>) -> Result<()> {
    // Determine the directory to analyze
    let target_dir = if let Some(dir) = dir {
        dir
    } else {
        // Default to current git repository root
        ".".to_string()
    };

    // Check if the directory is a git repository
    if !git::check_is_git_repo().await {
        return Err(anyhow::anyhow!("指定的目录不是git仓库"));
    }

    // Get project information
    let project_info = collect_project_info().await?;

    println!("{}", "🤖 正在分析项目结构...".cyan());
    
    // Get LLM client
    let llm_client = config::get_llm_client().await?;
    
    // Generate project understanding
    let understanding = generate_project_understanding(llm_client.as_client(), &project_info).await?;

    let skin = MadSkin::default();
    println!("\n{}\n", "=".repeat(60));
    skin.print_text(&understanding);
    println!("\n{}\n", "=".repeat(60));

    Ok(())
}

/// Collects project information for understanding.
async fn collect_project_info() -> Result<ProjectInfo> {
    // Get recent commits
    let recent_commits = git::run_git_command(&["log", "--oneline", "-10"]).await?;

    // Get file structure with filtering - use ls-files with --cached to get only existing files
    let all_files = git::run_git_command(&["ls-files", "--cached"]).await?;
    
    // Get all file statuses including deleted files
    let status_output = git::run_git_command(&["status", "--porcelain"]).await?;
    let deleted_files: std::collections::HashSet<String> = status_output
        .lines()
        .filter(|line| line.starts_with("D ") || line.starts_with(" D"))
        .map(|line| {
            // Handle both staged and unstaged deletions
            if line.starts_with("D ") {
                line[2..].trim().to_string()
            } else {
                line[1..].trim().to_string()
            }
        })
        .collect();
    
    // Also check for files that exist in git but not in filesystem
    let mut actually_existing_files = Vec::new();
    for file in all_files.lines() {
        let file = file.trim();
        if file.is_empty() {
            continue;
        }
        
        // Skip if file is marked as deleted
        if deleted_files.contains(file) {
            continue;
        }
        
        // Check if file actually exists in filesystem
        if std::path::Path::new(file).exists() {
            actually_existing_files.push(file.to_string());
        }
    }
    
    let filtered_files: Vec<String> = actually_existing_files
        .into_iter()
        .filter(|line| {
            // Filter out files that start with a dot (hidden files/dirs)
            if line.starts_with('.') {
                return false;
            }
            
            // Filter out dependency directories
            if line.contains("/node_modules/") ||
               line.contains("\\node_modules\\") ||
               line.contains("/target/") ||
               line.contains("\\target\\") ||
               line.contains("/venv/") ||
               line.contains("\\venv\\") ||
               line.contains("/__pycache__/") {
                return false;
            }
            
            // Filter out dependency and lock files
            let lower_line = line.to_lowercase();
            if lower_line.ends_with("cargo.lock") ||
               lower_line.ends_with("package-lock.json") ||
               lower_line.ends_with("yarn.lock") ||
               lower_line.ends_with("pnpm-lock.yaml") ||
               lower_line.ends_with("composer.lock") ||
               lower_line.ends_with("gemfile.lock") ||
               (lower_line.ends_with(".lock") && lower_line != "lock.toml" && lower_line != "lock.json") {
                return false;
            }
            
            // Filter out other unwanted files
            if lower_line.ends_with(".log") ||
               lower_line.ends_with(".tmp") ||
               lower_line.ends_with(".temp") ||
               lower_line.ends_with(".swp") ||
               lower_line.ends_with(".swo") {
                return false;
            }
            
            true
        })
        .collect();
    
    let file_structure = filtered_files.join("\n");

    // Read content of key files
    let mut file_contents = std::collections::HashMap::new();
    for file in &filtered_files {
        // Only read content of specific file types
        if file.ends_with(".md") || 
           file.ends_with(".txt") || 
           file.ends_with(".rs") ||
           file.ends_with(".toml") ||
           file.contains("README") ||
           file.contains("readme") {
            // Double-check file exists before reading
            if std::path::Path::new(file).exists() {
                // Limit content reading to avoid oversized prompts
                if let Ok(content) = read_file_content(file, 2000).await {
                    file_contents.insert(file.clone(), content);
                }
            }
        }
    }

    // Get project name from git
    let project_name = git::get_git_repo_name().await.unwrap_or_else(|_| "Unknown".to_string());

    // Get project type (simplified detection)
    let project_type = detect_project_type().await;

    // Get tech stack (simplified detection)
    let tech_stack = detect_tech_stack().await;

    // Get key features by analyzing source code
    let key_features = analyze_key_features(&filtered_files).await;

    Ok(ProjectInfo {
        name: project_name,
        project_type,
        tech_stack,
        file_structure,
        recent_commits,
        key_features,
        file_contents,
    })
}

/// Analyzes source code files to determine key features of the project
async fn analyze_key_features(files: &[String]) -> String {
    let mut features = Vec::new();
    
    // Look for common patterns that indicate features
    for file in files {
        if file.ends_with(".rs") {
            // For Rust files, look for command implementations
            if file.contains("commands") {
                features.push("CLI命令行工具".to_string());
            }
            if file.contains("git") {
                features.push("Git版本控制集成".to_string());
            }
            if file.contains("llm") {
                features.push("大语言模型集成".to_string());
            }
        } else if file.ends_with(".toml") && file.contains("config") {
            features.push("配置文件管理".to_string());
        }
    }
    
    // Add some default features if none were found
    if features.is_empty() {
        features.push("需要通过AI分析源码来确定".to_string());
    }
    
    features.join(", ")
}

/// Detects project type based on files in the repository.
async fn detect_project_type() -> String {
    // Check for actual existing files
    if std::path::Path::new("Cargo.toml").exists() {
        "Rust 项目".to_string()
    } else if std::path::Path::new("package.json").exists() {
        "Node.js 项目".to_string()
    } else if std::path::Path::new("requirements.txt").exists() || 
              std::path::Path::new("pyproject.toml").exists() ||
              std::path::Path::new("setup.py").exists() {
        "Python 项目".to_string()
    } else if std::path::Path::new("pom.xml").exists() {
        "Java 项目".to_string()
    } else if std::path::Path::new("go.mod").exists() {
        "Go 项目".to_string()
    } else {
        "未知类型项目".to_string()
    }
}

/// Detects technology stack based on files in the repository.
async fn detect_tech_stack() -> String {
    // Check for actual existing files
    let mut tech_stack = Vec::new();
    
    if std::path::Path::new("Cargo.toml").exists() {
        tech_stack.push("Rust".to_string());
    }
    
    if std::path::Path::new("package.json").exists() {
        tech_stack.push("JavaScript/TypeScript".to_string());
    }
    
    if std::path::Path::new("requirements.txt").exists() || 
       std::path::Path::new("pyproject.toml").exists() ||
       std::path::Path::new("setup.py").exists() {
        tech_stack.push("Python".to_string());
    }
    
    if std::path::Path::new("pom.xml").exists() {
        tech_stack.push("Java".to_string());
    }
    
    if std::path::Path::new("go.mod").exists() {
        tech_stack.push("Go".to_string());
    }
    
    if tech_stack.is_empty() {
        "未知技术栈".to_string()
    } else {
        tech_stack.join(", ")
    }
}

/// Generates project understanding using LLM.
async fn generate_project_understanding(client: &dyn LLMClient, project_info: &ProjectInfo) -> Result<String> {
    let template = config::get_prompt_template("understand").await?;
    let (system_prompt, user_prompt) = parse_prompt_template(&template)?;

    let progress_bar = ProgressBar::new_spinner();
    progress_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} AI is analyzing the project...")
            .unwrap(),
    );
    progress_bar.enable_steady_tick(std::time::Duration::from_millis(100));

    // Prepare context variables
    let project_context = format!(
        "项目名称: {}\n项目类型: {}\n技术栈: {}\n\n文件结构:\n{}\n\n最近提交记录:\n{}\n\n主要特性:\n{}",
        project_info.name,
        project_info.project_type,
        project_info.tech_stack,
        project_info.file_structure,
        project_info.recent_commits,
        project_info.key_features
    );

    // Create a summary of the file structure
    let file_structure_lines: Vec<&str> = project_info.file_structure.lines().collect();
    let file_structure_summary = if file_structure_lines.len() > 50 {
        // If there are too many files, just show the first 30 and last 20
        let first_part = file_structure_lines[..30].join("\n");
        let last_part = file_structure_lines[file_structure_lines.len()-20..].join("\n");
        format!("{}\n...\n{}", first_part, last_part)
    } else {
        project_info.file_structure.clone()
    };

    // Format file contents for the prompt
    let mut file_contents_str = String::new();
    for (file_path, content) in &project_info.file_contents {
        file_contents_str.push_str(&format!("\n文件: {}\n{}\n", file_path, content));
    }

    let final_prompt = user_prompt
        .replace("{project_name}", &project_info.name)
        .replace("{project_type}", &project_info.project_type)
        .replace("{tech_stack}", &project_info.tech_stack)
        .replace("{file_structure_summary}", &file_structure_summary)
        .replace("{key_features}", &project_info.key_features)
        .replace("{recent_changes}", &project_info.recent_commits)
        .replace("{project_context}", &project_context)
        .replace("{file_contents}", &file_contents_str);

    let understanding = client.call(&system_prompt, &final_prompt).await;
    progress_bar.finish_with_message("✓ AI analysis complete");
    understanding
}

/// Reads the content of a file, limited to a maximum number of characters.
async fn read_file_content(file_path: &str, max_chars: usize) -> Result<String> {
    use tokio::fs;
    
    // Read the file content
    let content = fs::read_to_string(file_path).await?;
    
    // Limit the content to the specified number of characters
    if content.len() > max_chars {
        Ok(content.chars().take(max_chars).collect::<String>() + "\n... (content truncated)")
    } else {
        Ok(content)
    }
}

/// Project information structure.
struct ProjectInfo {
    name: String,
    project_type: String,
    tech_stack: String,
    file_structure: String,
    recent_commits: String,
    key_features: String,
    file_contents: HashMap<String, String>,
}