//! src/commands/understand.rs

use crate::config;
use crate::git;
use crate::llm::{parse_prompt_template, LLMClient};
use anyhow::Result;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use termimad::MadSkin;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

/// Handles the project understanding process.
pub async fn handle_understand(_dir: Option<String>) -> Result<()> {
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

/// Get recent git commits for project context
async fn get_recent_commits() -> Result<String> {
    // Get last 5 commits with their messages and dates
    let commits = git::run_git_command(&["log", "--oneline", "-5", "--pretty=format:%h %s (%cr)"]).await?;
    Ok(commits)
}

/// Collects project information for understanding.
async fn collect_project_info() -> Result<ProjectInfo> {
    // Get recent commits for context
    let recent_commits = get_recent_commits().await.unwrap_or_else(|_| "无法获取提交记录".to_string());

    // Scan the actual filesystem structure instead of git files
    let filtered_files = scan_filesystem_structure().await?;
    
    let file_structure = filtered_files.join("\n");

    // Read content of all relevant files
    let mut file_contents = std::collections::HashMap::new();
    for file in &filtered_files {
        // Read content of all relevant files
        if is_relevant_file(file) {
            // Double-check file exists before reading
            if std::path::Path::new(file).exists() {
                // Read file content with increased limit
                if let Ok(content) = read_file_content(file).await {
                    file_contents.insert(file.clone(), content);
                }
            }
        }
    }

    // Get project name from current directory
    let project_name = std::env::current_dir()
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Unknown".to_string());

    // Get project type (simplified detection)
    let project_type = detect_project_type().await;

    // Get tech stack (simplified detection)
    let tech_stack = detect_tech_stack().await;

    // Get key features by analyzing actual file contents
    let key_features = analyze_key_features_from_content(&file_contents).await;

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

/// Analyzes actual file contents to determine key features of the project
async fn analyze_key_features_from_content(file_contents: &HashMap<String, String>) -> String {
    let mut features = std::collections::HashSet::new();
    
    // Analyze actual file contents for features
    for (file_path, content) in file_contents {
        let content_lower = content.to_lowercase();
        
        // Detect CLI features
        if content_lower.contains("clap") || content_lower.contains("structopt") || 
           content_lower.contains("command") || content_lower.contains("args") {
            features.insert("CLI命令行工具".to_string());
        }
        
        // Detect Git integration
        if content_lower.contains("git") && (content_lower.contains("commit") || 
           content_lower.contains("diff") || content_lower.contains("repo")) {
            features.insert("Git版本控制集成".to_string());
        }
        
        // Detect LLM integration
        if content_lower.contains("llm") || content_lower.contains("openai") || 
           content_lower.contains("anthropic") || content_lower.contains("claude") {
            features.insert("大语言模型集成".to_string());
        }
        
        // Detect web server features
        if content_lower.contains("axum") || content_lower.contains("warp") || 
           content_lower.contains("actix") || content_lower.contains("rocket") {
            features.insert("Web服务器".to_string());
        }
        
        // Detect database features
        if content_lower.contains("sqlx") || content_lower.contains("diesel") || 
           content_lower.contains("sea-orm") || content_lower.contains("database") {
            features.insert("数据库集成".to_string());
        }
        
        // Detect async features
        if content_lower.contains("async") || content_lower.contains("tokio") || 
           content_lower.contains("futures") {
            features.insert("异步编程".to_string());
        }
        
        // Detect configuration management
        if file_path.ends_with(".toml") || content_lower.contains("serde") || 
           content_lower.contains("config") {
            features.insert("配置文件管理".to_string());
        }
        
        // Detect error handling
        if content_lower.contains("anyhow") || content_lower.contains("thiserror") || 
           content_lower.contains("error") {
            features.insert("错误处理".to_string());
        }
        
        // Detect logging
        if content_lower.contains("log") || content_lower.contains("tracing") || 
           content_lower.contains("env_logger") {
            features.insert("日志记录".to_string());
        }
    }
    
    if features.is_empty() {
        "基于文件内容分析未发现明确特性".to_string()
    } else {
        features.into_iter().collect::<Vec<_>>().join(", ")
    }
}

/// Detects project type based on actual files in the filesystem.
async fn detect_project_type() -> String {
    // Check for actual existing files
    if Path::new("Cargo.toml").exists() {
        "Rust 项目".to_string()
    } else if Path::new("package.json").exists() {
        "Node.js 项目".to_string()
    } else if Path::new("requirements.txt").exists() || 
              Path::new("pyproject.toml").exists() ||
              Path::new("setup.py").exists() {
        "Python 项目".to_string()
    } else if Path::new("pom.xml").exists() {
        "Java 项目".to_string()
    } else if Path::new("go.mod").exists() {
        "Go 项目".to_string()
    } else {
        "未知类型项目".to_string()
    }
}

/// Detects technology stack based on actual files in the filesystem.
async fn detect_tech_stack() -> String {
    // Check for actual existing files
    let mut tech_stack = Vec::new();
    
    if Path::new("Cargo.toml").exists() {
        tech_stack.push("Rust".to_string());
    }
    
    if Path::new("package.json").exists() {
        tech_stack.push("JavaScript/TypeScript".to_string());
    }
    
    if Path::new("requirements.txt").exists() || 
       Path::new("pyproject.toml").exists() ||
       Path::new("setup.py").exists() {
        tech_stack.push("Python".to_string());
    }
    
    if Path::new("pom.xml").exists() {
        tech_stack.push("Java".to_string());
    }
    
    if Path::new("go.mod").exists() {
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

/// Reads the content of a file, with a higher limit for better project understanding.
async fn read_file_content(file_path: &str) -> Result<String> {
    use tokio::fs;
    
    // Read the file content with a higher limit (10,000 characters instead of 2,000)
    let content = fs::read_to_string(file_path).await?;
    
    // Increased limit for better project understanding
    const MAX_CHARS: usize = 10000;
    
    if content.len() > MAX_CHARS {
        Ok(content.chars().take(MAX_CHARS).collect::<String>() + "\n... (content truncated)")
    } else {
        Ok(content)
    }
}

/// Scans the filesystem structure to get actual project files
async fn scan_filesystem_structure() -> Result<Vec<String>> {
    let mut files = Vec::new();
    scan_directory_recursive(".", &mut files, 0, 3)?; // Max depth 3
    Ok(files)
}

/// Recursively scans a directory for relevant files
fn scan_directory_recursive(
    dir_path: &str,
    files: &mut Vec<String>,
    current_depth: usize,
    max_depth: usize,
) -> Result<()> {
    if current_depth > max_depth {
        return Ok(());
    }

    let mut entries = std::fs::read_dir(dir_path)?;
    while let Some(entry) = entries.next() {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Skip hidden files and directories
        if file_name.starts_with('.') {
            continue;
        }

        // Skip common build/cache directories
        if file_name == "target" || 
           file_name == "node_modules" || 
           file_name == "__pycache__" ||
           file_name == "venv" ||
           file_name == ".git" {
            continue;
        }

        let relative_path = path.strip_prefix(".")
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        if path.is_dir() {
            // Recursively scan subdirectories
            scan_directory_recursive(&relative_path, files, current_depth + 1, max_depth)?;
        } else {
            // Check if it's a relevant file
            if is_relevant_file(&relative_path) {
                files.push(relative_path);
            }
        }
    }

    Ok(())
}

/// Determines if a file is relevant for project analysis
fn is_relevant_file(file_path: &str) -> bool {
    let lower_path = file_path.to_lowercase();
    
    // Skip lock files and build artifacts
    if lower_path.ends_with("cargo.lock") ||
       lower_path.ends_with("package-lock.json") ||
       lower_path.ends_with("yarn.lock") ||
       lower_path.ends_with("pnpm-lock.yaml") ||
       lower_path.ends_with("composer.lock") ||
       lower_path.ends_with("gemfile.lock") ||
       lower_path.ends_with(".log") ||
       lower_path.ends_with(".tmp") ||
       lower_path.ends_with(".temp") ||
       lower_path.ends_with(".swp") ||
       lower_path.ends_with(".swo") {
        return false;
    }

    // Include source code files, config files, and documentation
    lower_path.ends_with(".rs") ||
    lower_path.ends_with(".toml") ||
    lower_path.ends_with(".json") ||
    lower_path.ends_with(".md") ||
    lower_path.ends_with(".txt") ||
    lower_path.ends_with(".yml") ||
    lower_path.ends_with(".yaml") ||
    lower_path.ends_with(".py") ||
    lower_path.ends_with(".js") ||
    lower_path.ends_with(".ts") ||
    lower_path.ends_with(".go") ||
    lower_path.ends_with(".java") ||
    lower_path.ends_with(".cpp") ||
    lower_path.ends_with(".c") ||
    lower_path.ends_with(".h") ||
    lower_path.ends_with(".hpp") ||
    file_path.contains("README") ||
    file_path.contains("readme") ||
    file_path.contains("LICENSE") ||
    file_path.contains("CHANGELOG") ||
    file_path.contains("CONTRIBUTING")
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