use super::{ProjectContext, ProjectStructure, FileContext};
use crate::config;
use crate::git;

use anyhow::Result;
use ignore::WalkBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// 项目分析器
pub struct ProjectAnalyzer {
    root_path: PathBuf,
    file_cache: HashMap<String, String>,
}

impl ProjectAnalyzer {
    pub async fn new() -> Result<Self> {
        let root_path = std::env::current_dir()?;

        Ok(Self {
            root_path,
            file_cache: HashMap::new(),
        })
    }
    
    /// 分析整个代码库
    pub async fn analyze_codebase(&self) -> Result<ProjectContext> {
        // 1. 扫描项目结构
        let structure = self.scan_project_structure().await?;

        // 2. 识别语言和框架
        let (language, framework) = self.detect_language_and_framework(&structure).await?;

        // 3. 分析关键文件
        let key_files = self.analyze_key_files(&structure).await?;

        // 4. 生成架构说明（包含项目树）
        let project_tree = self.generate_project_tree().await?;
        let architecture_notes = format!(
            "{}\n\n{}",
            self.generate_architecture_notes(&structure, &key_files).await?,
            project_tree
        );

        Ok(ProjectContext {
            language,
            framework,
            structure,
            key_files,
            architecture_notes,
        })
    }
    
    /// 扫描项目结构
    async fn scan_project_structure(&self) -> Result<ProjectStructure> {
        let mut directories = Vec::new();
        let mut entry_points = Vec::new();
        let mut patterns = Vec::new();

        // 使用 ignore::WalkBuilder 来正确处理 .gitignore 文件
        let walker = WalkBuilder::new(&self.root_path)
            .max_depth(Some(3))
            .build();

        for result in walker {
            let entry = result?;
            let path = entry.path();

            // 跳过项目根目录本身
            if path == self.root_path {
                continue;
            }

            let relative_path = path.strip_prefix(&self.root_path)?;

            if path.is_dir() {
                let dir_str = relative_path.to_string_lossy().to_string();
                if !dir_str.is_empty() {
                    directories.push(dir_str);
                }
            } else if self.is_entry_point(path) {
                let entry_str = relative_path.to_string_lossy().to_string();
                entry_points.push(entry_str);
            }
        }

        // 识别架构模式
        patterns = self.identify_patterns(&directories);

        Ok(ProjectStructure {
            directories,
            patterns,
            entry_points,
        })
    }

    /// 生成项目文件树
    pub async fn generate_project_tree(&self) -> Result<String> {
        let mut tree_lines = Vec::new();
        let mut file_count = 0;

        // 使用 WalkBuilder 遍历项目文件
        let walker = WalkBuilder::new(&self.root_path)
            .max_depth(Some(4)) // 稍微深一点以获取更多信息
            .build();

        let mut entries: Vec<_> = walker.collect::<Result<Vec<_>, _>>()?;

        // 按路径排序
        entries.sort_by(|a, b| a.path().cmp(b.path()));

        for entry in entries {
            let path = entry.path();

            // 跳过项目根目录本身
            if path == self.root_path {
                continue;
            }

            if let Ok(relative_path) = path.strip_prefix(&self.root_path) {
                let path_str = relative_path.to_string_lossy();
                let depth = relative_path.components().count();

                // 生成缩进
                let indent = "  ".repeat(depth.saturating_sub(1));
                let prefix = if path.is_dir() { "📁" } else { "📄" };

                if let Some(file_name) = relative_path.file_name() {
                    tree_lines.push(format!("{}{} {}", indent, prefix, file_name.to_string_lossy()));
                    if path.is_file() {
                        file_count += 1;
                    }
                }
            }
        }

        // 限制输出长度以避免 token 限制
        if tree_lines.len() > 100 {
            tree_lines.truncate(100);
            tree_lines.push("... (更多文件已省略)".to_string());
        }

        let tree_content = tree_lines.join("\n");
        let header = format!("项目文件树 (共 {} 个文件):\n", file_count);

        Ok(format!("{}{}", header, tree_content))
    }
    

    
    /// 检查是否是入口文件
    fn is_entry_point(&self, path: &Path) -> bool {
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            matches!(file_name, 
                "main.rs" | "lib.rs" | "mod.rs" |
                "main.py" | "__init__.py" |
                "index.js" | "app.js" | "server.js" |
                "main.go" | "main.java" |
                "App.tsx" | "index.tsx"
            )
        } else {
            false
        }
    }
    
    /// 识别架构模式
    fn identify_patterns(&self, directories: &[String]) -> Vec<String> {
        let mut patterns = Vec::new();
        
        // 检查常见的架构模式
        if directories.iter().any(|d| d.contains("src/commands")) {
            patterns.push("Command Pattern".to_string());
        }
        if directories.iter().any(|d| d.contains("src/models") || d.contains("models/")) {
            patterns.push("MVC Pattern".to_string());
        }
        if directories.iter().any(|d| d.contains("src/services") || d.contains("services/")) {
            patterns.push("Service Layer".to_string());
        }
        if directories.iter().any(|d| d.contains("components/")) {
            patterns.push("Component-Based".to_string());
        }
        
        patterns
    }
    
    /// 检测语言和框架
    async fn detect_language_and_framework(&self, _structure: &ProjectStructure) -> Result<(String, Option<String>)> {
        // 检查配置文件来确定语言和框架
        let config_files = [
            ("Cargo.toml", "rust", Some("cargo")),
            ("package.json", "javascript", None),
            ("requirements.txt", "python", None),
            ("go.mod", "go", None),
            ("pom.xml", "java", Some("maven")),
        ];
        
        for (file, lang, framework) in config_files {
            let file_path = self.root_path.join(file);
            if file_path.exists() {
                // 进一步检测框架
                let detected_framework = if let Some(fw) = framework {
                    Some(fw.to_string())
                } else {
                    self.detect_framework(lang, &file_path).await?
                };
                
                return Ok((lang.to_string(), detected_framework));
            }
        }
        
        Ok(("unknown".to_string(), None))
    }
    
    /// 检测具体框架
    async fn detect_framework(&self, language: &str, config_file: &Path) -> Result<Option<String>> {
        let content = fs::read_to_string(config_file).await?;
        
        match language {
            "javascript" => {
                if content.contains("\"react\"") {
                    Ok(Some("react".to_string()))
                } else if content.contains("\"express\"") {
                    Ok(Some("express".to_string()))
                } else if content.contains("\"next\"") {
                    Ok(Some("nextjs".to_string()))
                } else {
                    Ok(None)
                }
            }
            "rust" => {
                if content.contains("actix-web") {
                    Ok(Some("actix-web".to_string()))
                } else if content.contains("axum") {
                    Ok(Some("axum".to_string()))
                } else if content.contains("rocket") {
                    Ok(Some("rocket".to_string()))
                } else {
                    Ok(None)
                }
            }
            "python" => {
                if content.contains("django") {
                    Ok(Some("django".to_string()))
                } else if content.contains("flask") {
                    Ok(Some("flask".to_string()))
                } else if content.contains("fastapi") {
                    Ok(Some("fastapi".to_string()))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
    
    /// 分析关键文件
    async fn analyze_key_files(&self, structure: &ProjectStructure) -> Result<Vec<FileContext>> {
        let mut key_files = Vec::new();
        
        // 分析入口文件
        for entry_point in &structure.entry_points {
            let file_path = self.root_path.join(entry_point);
            if let Ok(context) = self.analyze_single_file(&file_path).await {
                key_files.push(context);
            }
        }
        
        // 分析配置文件
        let config_files = ["Cargo.toml", "package.json", "requirements.txt"];
        for config_file in config_files {
            let file_path = self.root_path.join(config_file);
            if file_path.exists() {
                if let Ok(context) = self.analyze_single_file(&file_path).await {
                    key_files.push(context);
                }
            }
        }
        
        Ok(key_files)
    }
    
    /// 分析单个文件
    async fn analyze_single_file(&self, file_path: &Path) -> Result<FileContext> {
        let content = fs::read_to_string(file_path).await?;
        let relative_path = file_path.strip_prefix(&self.root_path)?;
        
        // 这里可以使用 LLM 来分析文件内容，生成摘要
        // 为了简化，先使用基本的分析
        let summary = self.generate_file_summary(&content, file_path).await?;
        let key_functions = self.extract_key_functions(&content, file_path);
        let dependencies = self.extract_dependencies(&content, file_path);
        
        Ok(FileContext {
            path: relative_path.to_string_lossy().to_string(),
            summary,
            key_functions,
            dependencies,
        })
    }
    
    /// 生成文件摘要
    async fn generate_file_summary(&self, content: &str, file_path: &Path) -> Result<String> {
        // 如果文件太大，安全地截取前1000个字符（考虑字符边界）
        let _content_preview = if content.len() > 1000 {
            // 找到安全的字符边界
            let mut boundary = 1000;
            while boundary > 0 && !content.is_char_boundary(boundary) {
                boundary -= 1;
            }
            &content[..boundary]
        } else {
            content
        };
        
        // 基于文件扩展名和内容生成简单摘要
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            match ext {
                "rs" => Ok(format!("Rust源文件，包含{}行代码", content.lines().count())),
                "js" | "ts" => Ok(format!("JavaScript/TypeScript文件，包含{}行代码", content.lines().count())),
                "py" => Ok(format!("Python文件，包含{}行代码", content.lines().count())),
                "toml" | "json" => Ok("配置文件".to_string()),
                _ => Ok(format!("文件类型: {}", ext)),
            }
        } else {
            Ok("未知文件类型".to_string())
        }
    }
    
    /// 提取关键函数
    fn extract_key_functions(&self, content: &str, file_path: &Path) -> Vec<String> {
        let mut functions = Vec::new();
        
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            match ext {
                "rs" => {
                    // 简单的 Rust 函数提取
                    for line in content.lines() {
                        if line.trim().starts_with("pub fn ") || line.trim().starts_with("fn ") {
                            if let Some(func_name) = line.split_whitespace().nth(1) {
                                functions.push(func_name.split('(').next().unwrap_or(func_name).to_string());
                            }
                        }
                    }
                }
                "js" | "ts" => {
                    // 简单的 JavaScript 函数提取
                    for line in content.lines() {
                        if line.contains("function ") || line.contains("const ") && line.contains("=>") {
                            // 简化的函数名提取
                            functions.push("JavaScript函数".to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        
        functions
    }
    
    /// 提取依赖关系
    fn extract_dependencies(&self, content: &str, file_path: &Path) -> Vec<String> {
        let mut dependencies = Vec::new();
        
        // 提取 import/use 语句
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("use ") || trimmed.starts_with("import ") {
                dependencies.push(trimmed.to_string());
            }
        }
        
        dependencies
    }
    
    /// 生成架构说明
    async fn generate_architecture_notes(&self, structure: &ProjectStructure, key_files: &[FileContext]) -> Result<String> {
        let mut notes = String::new();
        
        notes.push_str(&format!("项目包含 {} 个目录，", structure.directories.len()));
        notes.push_str(&format!("{} 个入口文件。", structure.entry_points.len()));
        
        if !structure.patterns.is_empty() {
            notes.push_str(&format!("\n识别的架构模式: {}", structure.patterns.join(", ")));
        }
        
        notes.push_str(&format!("\n关键文件数量: {}", key_files.len()));
        
        Ok(notes)
    }
    
    /// 根据需求描述找到相关文件
    pub async fn find_related_files(&self, description: &str) -> Result<Vec<FileContext>> {
        // 这里可以使用更智能的方法，比如向量搜索
        // 现在先使用简单的关键词匹配
        let key_files = self.analyze_key_files(&self.scan_project_structure().await?).await?;

        // 简单的关键词匹配
        let related_files = key_files.into_iter()
            .filter(|file| {
                description.to_lowercase().contains("auth") && file.path.contains("auth") ||
                description.to_lowercase().contains("user") && file.path.contains("user") ||
                description.to_lowercase().contains("api") && file.path.contains("api")
            })
            .collect();

        Ok(related_files)
    }

    /// 获取压缩的项目上下文（用于处理 token 限制）
    pub async fn get_compressed_context(&self, max_files: usize) -> Result<ProjectContext> {
        let mut context = self.analyze_codebase().await?;

        // 限制关键文件数量
        if context.key_files.len() > max_files {
            context.key_files.truncate(max_files);
        }

        // 压缩架构说明
        if context.architecture_notes.len() > 500 {
            context.architecture_notes = format!("{}...(已压缩)", &context.architecture_notes[..500]);
        }

        // 限制目录数量
        if context.structure.directories.len() > 20 {
            context.structure.directories.truncate(20);
        }

        Ok(context)
    }

    /// 获取精简的相关文件列表
    pub async fn get_essential_related_files(&self, description: &str, max_files: usize) -> Result<Vec<FileContext>> {
        let mut related_files = self.find_related_files(description).await?;

        // 限制文件数量
        if related_files.len() > max_files {
            related_files.truncate(max_files);
        }

        // 压缩文件摘要
        for file in &mut related_files {
            if file.summary.len() > 200 {
                file.summary = format!("{}...", &file.summary[..200]);
            }

            // 限制函数列表
            if file.key_functions.len() > 5 {
                file.key_functions.truncate(5);
            }

            // 限制依赖列表
            if file.dependencies.len() > 3 {
                file.dependencies.truncate(3);
            }
        }

        Ok(related_files)
    }
}
