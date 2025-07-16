//! src/config.rs

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::llm::LLM;

/// Factory功能，根据配置获取LLM客户端。
pub async fn get_llm_client() -> Result<LLM> {
    let config = load_config().await?;
    crate::llm::create_llm_client(&config)
}

/// Returns the configuration directory path (~/.config/matecode).
pub async fn get_config_dir() -> Result<PathBuf> {
    let config_dir = if cfg!(windows) {
        // Windows: %APPDATA%\matecode
        dirs::data_dir()
            .map(|p| p.join("matecode"))
            .context("Could not get data directory")?
    } else {
        // Linux/macOS: ~/.config/matecode
        dirs::config_dir()
            .map(|p| p.join("matecode"))
            .context("Could not get config directory")?
    };

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .await
            .context("Could not create config directory")?;
    }
    Ok(config_dir)
}

/// Represents the main configuration for the application.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// The default LLM provider.
    pub provider: String,
    /// Language for prompts and UI
    pub language: String,
    /// LLM provider settings.
    pub llm: LLMProviders,
}

/// Defines the context window configuration for different models.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    /// The maximum number of tokens to use for the context.
    pub max_tokens: usize,
    /// The maximum number of tokens for the output.
    pub max_output_tokens: usize,
    /// Reserved tokens for system prompt and other overhead.
    pub reserved_tokens: usize,
}

/// Defines all LLM providers and their configurations.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LLMProviders {
    pub openai: Option<OpenAIProvider>,
    pub gemini: Option<GeminiProvider>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAIProvider {
    pub api_key: String,
    pub api_base: Option<String>,
    pub models: HashMap<String, ModelConfig>,
    pub default_model: String,
    pub proxy: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeminiProvider {
    pub api_key: String,
    pub models: HashMap<String, ModelConfig>,
    pub default_model: String,
    pub proxy: Option<String>,
}

/// Creates a default configuration file and directory structure.
pub async fn create_default_config() -> Result<()> {
    let config_dir = get_config_dir().await?;
    let config_path = config_dir.join("config.toml");
    
    // Create prompts directory
    let prompts_dir = config_dir.join("prompts");
    if !prompts_dir.exists() {
        fs::create_dir_all(&prompts_dir).await?;
    }

    // 只在配置文件不存在时才创建
    if !config_path.exists() {
        // 只保留必要的模型配置
        let mut openai_models = HashMap::new();
        
        // 私有化部署模型的通用配置
        openai_models.insert("default".to_string(), ModelConfig {
            max_tokens: 32_768,      // 大多数私有化模型的常见配置
            max_output_tokens: 4_096,
            reserved_tokens: 1_000,
        });

        let mut gemini_models = HashMap::new();
        
        // Gemini 2.5 Flash 配置
        gemini_models.insert("gemini-2.0-flash-exp".to_string(), ModelConfig {
            max_tokens: 1_048_576,   // Gemini 2.5 Flash 的实际参数
            max_output_tokens: 8_192,
            reserved_tokens: 2_000,
        });

        let default_config = Config {
            provider: "openai".to_string(),
            language: "zh-CN".to_string(),
            llm: LLMProviders {
                openai: Some(OpenAIProvider {
                    api_key: "YOUR_OPENAI_API_KEY".to_string(),
                    api_base: Some("http://localhost:8000/v1".to_string()),
                    models: openai_models,
                    default_model: "qwen2.5-72b-instruct".to_string(),
                    proxy: None,
                }),
                gemini: Some(GeminiProvider {
                    api_key: "YOUR_GEMINI_API_KEY".to_string(),
                    models: gemini_models,
                    default_model: "gemini-2.0-flash-exp".to_string(),
                    proxy: None,
                }),
            },
        };

        let config_content = toml::to_string_pretty(&default_config)?;
        let mut file = fs::File::create(&config_path).await?;
        file.write_all(config_content.as_bytes()).await?;
        
        println!("✅ 已创建默认配置文件: {:?}", config_path);
    } else {
        println!("⚠️  配置文件已存在，跳过创建: {:?}", config_path);
    }

    // 创建默认提示词模板（只在不存在时创建）
    create_default_prompts(&prompts_dir).await?;

    println!("✅ 已创建提示词模板目录: {:?}", prompts_dir);
    println!("\n📝 请编辑配置文件，设置您的 API 密钥:");
    println!("   {}", config_path.display());
    println!("\n💡 提示：私有化部署模型会自动使用 'default' 配置，无需手动添加每个模型。");
    
    Ok(())
}

pub async fn load_config() -> Result<Config> {
    let config_dir = get_config_dir().await?;
    let config_path = config_dir.join("config.toml");

    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "配置文件不存在。请先运行 'matecode init' 创建默认配置。"
        ));
    }

    let config_content = fs::read_to_string(config_path)
        .await
        .context("无法读取配置文件")?;
    let config: Config =
        toml::from_str(&config_content).context("配置文件格式错误")?;

    // Validate configuration
    validate_config(&config)?;

    Ok(config)
}

fn validate_config(config: &Config) -> Result<()> {
    match config.provider.as_str() {
        "openai" => {
            if let Some(openai) = &config.llm.openai {
                if openai.api_key == "YOUR_OPENAI_API_KEY" {
                    return Err(anyhow::anyhow!(
                        "请在配置文件中设置有效的 OpenAI API 密钥"
                    ));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "选择了 OpenAI 提供商，但未配置 OpenAI 设置"
                ));
            }
        }
        "gemini" => {
            if let Some(gemini) = &config.llm.gemini {
                if gemini.api_key == "YOUR_GEMINI_API_KEY" {
                    return Err(anyhow::anyhow!(
                        "请在配置文件中设置有效的 Gemini API 密钥"
                    ));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "选择了 Gemini 提供商，但未配置 Gemini 设置"
                ));
            }
        }
        _ => {
            return Err(anyhow::anyhow!(
                "不支持的 LLM 提供商: {}",
                config.provider
            ));
        }
    }
    Ok(())
}

async fn create_default_prompts(prompts_dir: &PathBuf) -> Result<()> {
    // 定义所有提示词模板
    let prompt_templates = vec![
        ("commit.toml", get_commit_prompt_template()),
        ("review.toml", get_review_prompt_template()),
        ("report.toml", get_report_prompt_template()),
        ("summarize.toml", get_summarize_prompt_template()),
        ("combine.toml", get_combine_prompt_template()),
    ];

    for (filename, content) in prompt_templates {
        let file_path = prompts_dir.join(filename);
        
        // 只在文件不存在时才创建
        if !file_path.exists() {
            fs::write(&file_path, content).await?;
            println!("✅ 已创建提示词模板: {:?}", file_path);
        } else {
            println!("⚠️  提示词模板已存在，跳过创建: {:?}", file_path);
        }
    }

    Ok(())
}

fn get_commit_prompt_template() -> &'static str {
    r#"[system]
你是一位专业的 Git commit message 编写专家，你的目标是生成读起来像人类工程师编写的 commit message。你的回应**只能**包含 commit message 内容，不要有其他任何解释。严格遵守 Conventional Commits 规范，但描述部分使用中文。

[user]
请根据以下的项目上下文和 git diff 内容生成一个中文 git commit message。

<project_context>
{project_tree}

本次修改影响的文件 ({total_files} 个):
{affected_files}
</project_context>

<rules>
1.  **Header (第一行)**:
    -   `type` 使用英文 (如 feat, fix, chore)。
    -   `scope` (可选) 概括变更涉及的模块。
    -   `subject` (主题) 必须用清晰的中文简明扼要地描述变更内容，不超过50个字符。
2.  **Body (正文, 可选)**:
    -   正文应详细解释 **为什么** 需要这次变更，解决了什么问题。
    -   描述这次变更是 **如何** 实现的，特别是关键的实现思路。
    -   避免使用AI化的、过于正式的语言（例如，不要写 "本次提交新增了..."，而应该更直接地描述）。
3.  **输出**: 只输出被 <commit_message> 标签包裹的 commit message。
</rules>

<example_good>
<commit_message>
feat(api): 实现用户认证功能

用户认证是系统的核心安全保障。本次提交引入了基于 JWT 的认证机制。
- 使用 `jsonwebtoken` 库生成和验证 token。
- 在 `auth` 中间件中实现 token 校验逻辑。
</commit_message>
</example_good>

<diff_content>
{diff_content}
</diff_content>
"#
}

fn get_review_prompt_template() -> &'static str {
    r#"[system]
You are an expert code reviewer. Your task is to analyze a git diff and provide constructive feedback. Focus on identifying potential bugs, improving code quality, and ensuring best practices are followed. Be clear, concise, and provide actionable suggestions. Structure your review in Markdown format.

[user]
Please review the following code changes provided in the git diff format.

## Git Diff:
```diff
{diff_content}
```

## Review Guidelines:
1.  **Overall Assessment:** Start with a brief, high-level summary of the changes.
2.  **Identify Issues and Suggestions:** For each file, provide specific feedback. Refer to line numbers where possible.
    -   **[Logic]**: Potential bugs, race conditions, or logic errors.
    -   **[Style]**: Code style, naming conventions, readability.
    -   **[Best Practice]**: Suggestions for using language features or libraries more effectively.
    -   **[Comment]**: Questions or requests for clarification.
3.  **Use Markdown:** Structure the review using headings for each file and bullet points for individual comments.
4.  **Be Constructive:** Frame your feedback positively. The goal is to help improve the code, not to criticize.
5.  **Language**: The review should be in Chinese.

## Example Output:

### `src/main.rs`
- **[Logic] at line 42:** The current logic might not handle empty input gracefully. Consider adding a check at the beginning of the function.
- **[Style] at line 55:** The variable `temp_data` could be renamed to `user_profile` for better clarity.

### `src/utils.rs`
- **[Best Practice] at line 12:** Instead of manually building the path string, consider using `PathBuf::join()` for better cross-platform compatibility.

Please provide your review for the provided diff.
"#
}

fn get_report_prompt_template() -> &'static str {
    r#"[system]
You are a senior project manager responsible for writing concise, clear, and insightful work summaries. Your goal is to synthesize a list of raw git commit messages from multiple projects into a unified report that is easy for stakeholders to understand. Group related items, use clear headings, and focus on the accomplishments and outcomes, not just the raw commit messages.

[user]
Please generate a work summary report in Markdown format based on the following commit messages from {start_date} to {end_date}.
The commits are grouped by project.

## Raw Commits:
{commits}

## Instructions:
1.  **Analyze and Group:** Read through all the commit messages from all projects. Group them into logical categories (e.g., "Feature Development," "Bug Fixes," "Refactoring").
2.  **Summarize Each Group:** For each category, write a high-level summary of the work accomplished. Use bullet points to list the key changes. **Crucially, you must mention which project the change belongs to.**
3.  **Use Clear Headings:** Use Markdown headings (e.g., `### ✨ 新功能`) for each category.
4.  **Focus on Impact:** Rephrase the commit messages to focus on the "what" and "why."
5.  **Language:** The report should be in Chinese.

## Desired Output Format:

### ✨ 新功能
- [项目A] - 实现用户登录和注册功能。
- [项目B] - 新增了数据导出的 API.

### 🐛 问题修复
- [项目A] - 修复了特定场景下闪退的问题。

Please generate the report now.
"#
}

fn get_summarize_prompt_template() -> &'static str {
    r#"[system]
你是一个代码变更分析专家。你需要简洁地总结这个代码块的主要变更内容。你的回应**只能**包含被 <summary> 标签包裹的摘要。

[user]
请分析以下代码变更并生成简洁的中文摘要。

<context>
项目文件数: {total_files}
涉及文件: {chunk_files}
</context>

<diff>
{diff_content}
</diff>

请用中文总结这个代码块的主要变更，重点关注功能性改变。
**注意**：只需要描述变更内容，不要生成完整的commit message格式。

例如:
<summary>
添加了用户认证模块和登录功能，并重构了数据库连接逻辑。
</summary>
"#
}

fn get_combine_prompt_template() -> &'static str {
    r#"[system]
你是一个根据代码变更摘要生成 Conventional Commits 规范的 git commit message 的专家。你的回应应该**只能**包含被 <commit_message> 标签包裹的 commit message，不包含任何额外的解释或引言。

[user]
请根据以下的项目上下文和代码变更摘要，为我生成一个高质量的、人类可读的中文 git commit message。

<project_context>
{project_tree}

本次修改影响的文件 ({total_files} 个):
{affected_files}
</project_context>

<summaries>
{summaries}
</summaries>

<rules>
1.  **目标**: 不要创建一个简单的变更日志。你的目标是写一个**高层次的总结**，解释这次系列变更的**核心目的**和**主要实现**。
2.  **格式**: 严格遵守 Conventional Commits 规范。
3.  **输出**: 只输出被 <commit_message> 标签包裹的 commit message。
</rules>

<example>
<commit_message>
feat(history): 引入提交历史归档与日报生成功能

为了更好地追踪开发进度和自动化生成工作报告，本次引入了提交历史的自动归档机制。

此功能通过 `post-commit` Git 钩子实现，确保只有最终被采纳的 commit 才会被记录。新增的 `report` 命令可以调用 AI 服务，将每日的提交记录智能地汇总成一份结构化的工作日报。
</commit_message>
</example>
"#
}

pub async fn get_prompt_template(name: &str) -> Result<String> {
    let config_dir = get_config_dir().await?;
    let prompt_path = config_dir.join("prompts").join(format!("{}.toml", name));
    
    if !prompt_path.exists() {
        return Err(anyhow::anyhow!(
            "提示词模板文件不存在: {}。请运行 'matecode init' 重新创建。",
            prompt_path.display()
        ));
    }

    let content = fs::read_to_string(prompt_path).await?;
    Ok(content)
}
