//! src/config.rs

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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
    /// Linter commands for different languages.
    #[serde(default = "default_linters")]
    pub lint: HashMap<String, String>,
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
        openai_models.insert(
            "default".to_string(),
            ModelConfig {
                max_tokens: 16_384, // 大多数私有化模型的常见配置
                max_output_tokens: 4_096,
                reserved_tokens: 1_000,
            },
        );

        let mut gemini_models = HashMap::new();

        // Gemini 2.5 Flash 配置
        gemini_models.insert(
            "gemini-2.0-flash-exp".to_string(),
            ModelConfig {
                max_tokens: 1_048_576, // Gemini 2.5 Flash 的实际参数
                max_output_tokens: 8_192,
                reserved_tokens: 2_000,
            },
        );

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
            lint: default_linters(),
        };

        let config_content = toml::to_string_pretty(&default_config)?;
        let mut file = fs::File::create(&config_path).await?;
        file.write_all(config_content.as_bytes()).await?;

        println!("✅ 已创建默认配置文件: {config_path:?}");
    } else {
        println!("⚠️  配置文件已存在，跳过创建: {config_path:?}");
    }

    // 创建默认提示词模板（只在不存在时创建）
    create_default_prompts(&prompts_dir).await?;

    // 创建默认 .matecode-ignore 文件
    create_default_ignore_file(&config_dir).await?;

    println!("✅ 已创建提示词模板目录: {prompts_dir:?}");
    println!("\n📝 请编辑配置文件，设置您的 API 密钥:");
    println!("   {}", config_path.display());
    println!("\n💡 提示：私有化部署模型会自动使用 'default' 配置，无需手动添加每个模型。");

    Ok(())
}

async fn create_default_ignore_file(config_dir: &Path) -> Result<()> {
    let ignore_file_path = config_dir.join(".matecode-ignore");

    // 只在文件不存在时才创建
    if !ignore_file_path.exists() {
        let ignore_content = get_default_ignore_content();
        fs::write(&ignore_file_path, ignore_content).await?;
        println!("✅ 已创建默认忽略文件: {ignore_file_path:?}");
    } else {
        println!("⚠️  忽略文件已存在，跳过创建: {ignore_file_path:?}");
    }

    Ok(())
}

fn get_default_ignore_content() -> &'static str {
    r#"# matecode 忽略规则
# 这个文件定义了在生成项目上下文时应该忽略的文件和目录
# 语法与 .gitignore 相同

# 依赖目录
node_modules/
target/
.venv/
venv/
__pycache__/
.pytest_cache/
.mypy_cache/
.ruff_cache/

# 构建产物
build/
dist/
*.egg-info/
.gradle/
out/

# 日志文件
*.log
logs/

# 临时文件
*.tmp
*.temp
.DS_Store
Thumbs.db

# IDE 配置
.vscode/
.idea/
*.swp
*.swo
*~

# 系统文件
.git/
.svn/
.hg/

# 大型数据文件
*.db
*.sqlite
*.sqlite3
*.dump

# 媒体文件
*.mp4
*.avi
*.mkv
*.mp3
*.wav
*.flac
*.jpg
*.jpeg
*.png
*.gif
*.bmp
*.tiff
*.webp
*.ico

# 压缩文件
*.zip
*.tar
*.tar.gz
*.tar.bz2
*.tar.xz
*.rar
*.7z

# 文档文件（可选，根据需要调整）
*.pdf
*.doc
*.docx
*.ppt
*.pptx
*.xls
*.xlsx
"#
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
    let config: Config = toml::from_str(&config_content).context("配置文件格式错误")?;

    // Validate configuration
    validate_config(&config)?;

    Ok(config)
}

fn validate_config(config: &Config) -> Result<()> {
    match config.provider.as_str() {
        "openai" => {
            if let Some(openai) = &config.llm.openai {
                if openai.api_key == "YOUR_OPENAI_API_KEY" {
                    return Err(anyhow::anyhow!("请在配置文件中设置有效的 OpenAI API 密钥"));
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
                    return Err(anyhow::anyhow!("请在配置文件中设置有效的 Gemini API 密钥"));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "选择了 Gemini 提供商，但未配置 Gemini 设置"
                ));
            }
        }
        _ => {
            return Err(anyhow::anyhow!("不支持的 LLM 提供商: {}", config.provider));
        }
    }
    Ok(())
}

async fn create_default_prompts(prompts_dir: &Path) -> Result<()> {
    // 定义所有提示词模板
    let prompt_templates = vec![
        ("commit.toml", get_commit_prompt_template()),
        ("review.toml", get_review_prompt_template()),
        ("report.toml", get_report_prompt_template()),
        ("summarize.toml", get_summarize_prompt_template()),
        ("combine.toml", get_combine_prompt_template()),
        ("branch.toml", get_generate_branch_prompt_template()),
        ("plan_clarify.toml", get_plan_clarify_prompt_template()),
        ("plan_clarify_specific.toml", get_plan_clarify_specific_prompt_template()),
        ("plan_generate.toml", get_plan_generate_prompt_template()),
        ("doc_generate.toml", get_doc_generate_prompt_template()),
        ("diagram_generate.toml", get_diagram_generate_prompt_template()),
    ];

    for (filename, content) in prompt_templates {
        let file_path = prompts_dir.join(filename);

        // 只在文件不存在时才创建
        if !file_path.exists() {
            fs::write(&file_path, content).await?;
            println!("✅ 已创建提示词模板: {file_path:?}");
        } else {
            println!("⚠️  提示词模板已存在，跳过创建: {file_path:?}");
        }
    }

    Ok(())
}

fn get_commit_prompt_template() -> &'static str {
    r#"[system]
你是一位专业的 Git commit message 编写专家，你的目标是生成人类工程师编写的 commit message。你的回应**只能**包含 commit message 内容，不要有其他任何解释。严格遵守 Angular 规范，但描述部分使用中文。

**重要：语言要求**
{language_instruction}

[user]
请根据以下的项目上下文和 git diff 内容生成一个中文 git commit message。
你需要根据项目的改动信息，来生成一个考虑到对项目的影响，而不是只根据某个文件的改动生成一个简单的commit_message。

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

用户认证是系统的核心安全保障, 引入了基于 JWT 的认证机制。
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
你是一位经验丰富的代码审查专家。你的任务是帮助开发者发现代码中的问题并提供具体的改进建议。请用直接、实用的方式指出问题，不要客套话，重点关注代码质量、潜在问题和最佳实践。

**重要：语言要求**
{language_instruction}

[user]
请审查以下代码变更，重点关注以下几个方面：

<lint_results></lint_results>

```diff
{diff_content}
```

## 审查重点:

**🔍 必须检查的问题:**
1. **安全漏洞**: 是否存在安全风险？
2. **性能问题**: 是否有明显的性能瓶颈？
3. **逻辑错误**: 边界条件、空值处理、错误处理是否完善？
4. **资源泄漏**: 是否正确释放资源？

**📝 代码质量:**
1. **可读性**: 变量命名、函数结构是否清晰？
2. **重复代码**: 是否可以抽取公共逻辑？
3. **复杂度**: 函数是否过于复杂，需要拆分？
4. **一致性**: 是否符合项目的代码风格？

**⚡ 改进建议:**
1. **更好的实现方式**: 有没有更简洁或更高效的写法？
2. **最佳实践**: 是否遵循了语言/框架的最佳实践？
3. **可维护性**: 未来修改这段代码会不会很困难？

## 输出格式:
对于每个发现的问题，请按以下格式输出：

**文件: `路径/文件名`**
- **⚠️ [问题类型] 第X行:** 具体问题描述
- **💡 建议:** 具体的改进方案
- **🔧 示例:** (如果需要) 提供代码示例

**示例:**
**文件: `src/main.rs`**
- **⚠️ [安全] 第 15 行:** 直接使用用户输入构建 SQL 查询，存在 SQL 注入风险
- **💡 建议:** 使用参数化查询或 ORM 来避免 SQL 注入
- **🔧 示例:** `query("SELECT * FROM users WHERE id = ?", [user_id])`

- **⚠️ [性能] 第 32 行:** 在循环中重复调用数据库查询
- **💡 建议:** 将查询移出循环，或使用批量查询

如果代码质量很好，请简单说明哪些地方做得不错，然后重点指出还可以改进的地方。

**重要:** 请直接指出问题，不要过分客气。目标是帮助代码变得更好。
"#
}

fn get_report_prompt_template() -> &'static str {
    r#"[system]
你是一位开发者，你现在会阅读你最近提交的commit信息，并根据这些commit信息生成一份工作总结。你会使用清晰的标题，将成果和产出列出，而不是罗列原始的提交信息。

**重要：语言要求**
回答和思考保持使用语言: {language_instruction}

[user]
请根据以下从 {start_date} 到 {end_date} 的提交信息，生成一份 Markdown 格式的工作总结报告。
提交信息已按项目分组。

## 原始提交记录:
{commits}

## 指示:
1.  **分析与分组:** 阅读所有项目的全部提交信息。将它们按逻辑类别分组（例如，"功能开发"、"问题修复"、"代码重构"）。
2.  **总结每个分组:** 为每个类别撰写一个高层次的概要，总结所完成的工作。使用项目符号列出关键变更。**至关重要的是，你必须提及变更属于哪个项目。**
3.  **使用清晰的标题:** 为每个类别使用 Markdown 标题（例如，`### ✨ 新功能`）。
4.  **关注影响:** 重新表述提交信息，使其专注于"做了什么"和"为什么做"。
5.  **杜绝重复**： 不要出现重复的成果和产出, 比如新功能出现的内容肯定不要出现在其他主题内，请保持专业的态度来处理。
6.  **保持简洁**： 不要出现冗长的描述，你应该根据commit的信息保持合适的篇幅，比如7天的总结，你只需要保持一到两百字左右的描述即可。

## 期望的输出格式:

### ✨ 新功能
- [项目A] - 实现用户登录和注册功能。
- [项目B] - 新增了数据导出的 API。

### 🐛 问题修复
- [项目A] - 修复了特定场景下闪退的问题。

请立即生成报告。
"#
}

fn get_summarize_prompt_template() -> &'static str {
    r#"[system]
你是一个代码变更分析专家。你需要简洁地总结这个代码块的主要变更内容。你的回应**只能**包含被 <summary> 标签包裹的摘要。

**重要：语言要求**
{language_instruction}

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

**重要：语言要求**
{language_instruction}
 
[user]
请根据以下的项目上下文和代码变更摘要，为我生成一个高质量的、人类可读的中文 git commit message。
 
**请注意：**
*   你的目标是提供一个**高层次的总结**，解释本次系列变更的**核心目的**和**主要实现**，而不是简单地罗列每个文件的具体修改点。
*   将多个相关的重构或优化操作归纳为一个主要的改动点，并用简洁的语言描述其**整体价值**。
*   严格遵守 Conventional Commits 规范（例如：`feat:`, `fix:`, `refactor:`, `chore:`, `docs:`, `style:`, `test:`, `perf:`, `build:`, `ci:`, `revert:`）。
*   commit message 的主体部分应包含对本次变更的**简要描述**，说明为什么要做这些改动以及它们解决了什么问题。
*   如果可能，使用**动词开头**的简洁表述来概括主要改动。
 
<project_context>
 
{project_tree}
 
本次修改影响的文件 ({total_files} 个):
{affected_files}
 
</project_context>
 
 
<summaries>
 
{summaries}
 
</summaries>
 
<rules>
 
1.  **核心目的与主要实现**: 提炼本次系列变更的**核心目的**和**主要实现方式**，用一两句话概括。避免逐条列出文件或函数的修改。
2.  **Conventional Commits 规范**: 严格遵守 Conventional Commits 规范，包括类型（type）、作用域（scope，如果适用）和描述（subject）。
3.  **主体内容**: commit message 的主体部分应提供更详细的解释，说明本次变更的背景、原因和带来的好处。
4.  **语言风格**: 使用简洁、清晰、专业且易于理解的中文。
5.  **输出格式**: 只输出被 <commit_message> 标签包裹的 commit message。
 
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

fn get_generate_branch_prompt_template() -> &'static str {
    r#"[system]
你是我的 Git 分支命名专家。你的任务是根据我提供的功能描述，生成一个符合项目规范且清晰的分支名称。

**重要：语言要求**
{language_instruction}

**分支命名规范:**
- 使用 conventional commits 风格的前缀：feat/, fix/, docs/, style/, refactor/, test/, chore/
- 使用小写字母和连字符分隔单词
- 保持简洁但具有描述性
- 避免使用特殊字符和空格

**规则:**
1. 你的回应**只能**包含被 `<branch_name>` 标签包裹的分支名称
2. 不要有任何其他解释、说明或额外的文本

[user]
请根据以下功能描述生成合适的分支名称：

功能描述: {description}

{staged_context}

请生成一个符合规范的分支名称。

**示例:**
功能描述: 修复支付页面按钮点击无响应的 bug
输出: <branch_name>fix/payment-page-button-unresponsive</branch_name>
"#
}

pub async fn get_prompt_template(name: &str) -> Result<String> {
    let config_dir = get_config_dir().await?;
    let prompt_path = config_dir.join("prompts").join(format!("{name}.toml"));

    if !prompt_path.exists() {
        return Err(anyhow::anyhow!(
            "提示词模板文件不存在: {prompt_path:?}。请运行 'matecode init' 重新创建。",
        ));
    }

    let mut content = fs::read_to_string(prompt_path).await?;

    // 加载配置以获取语言设置
    let config = load_config().await?;
    let language_instruction = get_language_instruction(&config.language);

    // 在提示词中插入语言设置
    content = content.replace("{language_instruction}", &language_instruction);

    Ok(content)
}

fn get_language_instruction(language: &str) -> String {
    match language {
        "zh-CN" => "请务必使用简体中文回复。所有输出内容都应该是中文，包括技术术语的描述和解释。".to_string(),
        "en-US" => "Please respond in English. All output content should be in English, including technical terms and explanations.".to_string(),
        "ja-JP" => "日本語で回答してください。すべての出力内容は日本語で、技術用語の説明も含めて日本語で記述してください。".to_string(),
        "ko-KR" => "한국어로 답변해 주세요. 모든 출력 내용은 기술 용어 설명을 포함하여 한국어로 작성되어야 합니다.".to_string(),
        "fr-FR" => "Veuillez répondre en français. Tout le contenu de sortie doit être en français, y compris les descriptions de termes techniques.".to_string(),
        "de-DE" => "Bitte antworten Sie auf Deutsch. Alle Ausgabeinhalte sollten auf Deutsch sein, einschließlich der Beschreibungen technischer Begriffe.".to_string(),
        "es-ES" => "Por favor responda en español. Todo el contenido de salida debe estar en español, incluidas las descripciones de términos técnicos.".to_string(),
        "it-IT" => "Si prega di rispondere in italiano. Tutti i contenuti di output dovrebbero essere in italiano, comprese le descrizioni dei termini tecnici.".to_string(),
        "pt-BR" => "Por favor, responda em português. Todo o conteúdo de saída deve estar em português, incluindo descrições de termos técnicos.".to_string(),
        "ru-RU" => "Пожалуйста, отвечайте на русском языке. Все выходные данные должны быть на русском языке, включая описания технических терминов.".to_string(),
        _ => format!("Please respond in the language: {language}. All output content should be in this language, including technical terms and explanations."),
    }
}

fn default_linters() -> HashMap<String, String> {
    let mut linters = HashMap::new();
    linters.insert(
        "rust".to_string(),
        "cargo clippy -- -D warnings".to_string(),
    );
    linters.insert("python".to_string(), "ruff check .".to_string());
    linters.insert("javascript".to_string(), "eslint .".to_string());
    linters.insert("typescript".to_string(), "eslint .".to_string());
    linters.insert("go".to_string(), "go vet ./...".to_string());
    linters.insert(
        "java".to_string(),
        "# (需要配置) e.g., checkstyle -c /path/to/google_checks.xml .".to_string(),
    );
    linters.insert(
        "cpp".to_string(),
        "# (需要配置) e.g., clang-tidy **/*.cpp --".to_string(),
    );
    linters
}

fn get_plan_clarify_prompt_template() -> &'static str {
    r#"[system]
你是一位资深的产品经理和技术架构师。你的任务是通过苏格拉底式提问来澄清用户的模糊需求，帮助他们形成清晰、具体的需求描述。

你需要：
1. 深入理解用户的真实意图和业务目标
2. 识别技术实现的关键决策点
3. 发现可能的边界情况和约束条件
4. 确保需求的完整性和可实现性

请生成3-5个关键问题，每个问题都应该帮助澄清需求的重要方面。

**重要：语言要求**
{language_instruction}

[user]
用户提出的原始需求：{description}

请生成一系列澄清问题，帮助深入理解这个需求。问题应该涵盖：
- 业务目标和用户价值
- 功能边界和约束条件
- 技术实现的关键决策点
- 与现有系统的集成方式
- 性能和安全要求

请以列表形式输出问题，每行一个问题，使用 "- " 开头。

例如：
- 这个功能的主要目标用户是谁？
- 预期的并发用户数量是多少？
- 是否需要与现有的认证系统集成？
"#
}

fn get_plan_clarify_specific_prompt_template() -> &'static str {
    r#"[system]
你是一位技术专家。基于用户的需求描述，生成2-3个针对该特定需求的深度澄清问题。

这些问题应该：
1. 针对该需求的特定技术领域或业务场景
2. 深入挖掘实现细节和边界条件
3. 避免与通用问题重复

**重要：语言要求**
{language_instruction}

[user]
用户的具体需求：{description}

请基于这个需求的特点，生成2-3个深度澄清问题。问题应该针对：
- 该需求特有的技术挑战
- 具体的实现方式选择
- 特殊的业务规则或约束

请以列表形式输出，每行一个问题，使用 "- " 开头。

例如（针对"用户徽章系统"）：
- 徽章是否支持等级制度？比如铜牌、银牌、金牌？
- 徽章的展示位置有哪些？用户头像、个人主页、还是评论区？
- 是否需要徽章的获取历史记录和统计功能？
"#
}

fn get_plan_generate_prompt_template() -> &'static str {
    r#"[system]
你是一位经验丰富的技术架构师和项目经理。基于澄清后的需求信息，你需要生成一个详细的技术实施计划。

计划应该包括：
1. 清晰的技术方案描述
2. 详细的任务分解
3. 影响分析
4. 实施建议

**重要：语言要求**
{language_instruction}

[user]
**原始需求**: {original_description}

**澄清后的需求信息**:
{clarified_requirements}

请基于以上信息生成一个详细的技术实施计划。计划应该包括：

## 技术方案
描述整体的技术实现方案和架构设计

## 任务分解
将实施过程分解为具体的任务，每个任务应该包括：
- 任务标题
- 详细描述
- 预估工时
- 涉及的文件或模块
- 依赖关系

## 影响分析
分析这个需求对现有系统可能产生的影响

## 实施建议
提供实施过程中的注意事项和建议

请使用结构化的格式输出，便于后续解析和处理。
"#
}

fn get_doc_generate_prompt_template() -> &'static str {
    r#"[system]
你是一位技术文档专家。你需要基于提供的计划和代码分析结果，生成一份高质量的技术文档。

文档应该：
1. 结构清晰，逻辑性强
2. 包含必要的技术细节
3. 便于开发者理解和实施
4. 包含代码示例和最佳实践

**重要：语言要求**
{language_instruction}

[user]
请基于以下信息生成技术文档：

{context}

请生成一份包含以下章节的技术文档：

## 概述
简要描述功能目标和价值

## 技术方案
详细说明技术实现方案

## 系统架构
描述系统的整体架构和组件关系

## 核心业务流程
说明主要的业务流程和数据流

## 关键实现细节
重要的技术实现细节和注意事项

## 数据结构设计
相关的数据结构和接口设计

## 测试策略
测试方案和验收标准

## 部署和运维
部署流程和运维注意事项

请确保文档内容详实、准确，并包含必要的代码示例。
"#
}

fn get_diagram_generate_prompt_template() -> &'static str {
    r#"[system]
你是一位系统架构师和流程设计专家。你需要基于提供的上下文信息，生成相应的Mermaid图表代码。

你可以生成以下类型的图表：
1. 流程图 (flowchart) - 展示业务流程或算法流程
2. 时序图 (sequenceDiagram) - 展示组件间的交互时序
3. 类图 (classDiagram) - 展示类的结构和关系
4. 组件图 - 展示系统组件和依赖关系

**重要：语言要求**
{language_instruction}

[user]
请基于以下上下文信息生成合适的Mermaid图表：

{context}

请生成1-3个最能说明系统设计的图表。每个图表应该：
1. 有清晰的标题
2. 使用正确的Mermaid语法
3. 包含必要的说明

请使用以下格式输出：

## 图表标题
```mermaid
图表代码
```

例如：

## 用户认证流程
```mermaid
flowchart TD
    A[用户登录] --> B{验证凭据}
    B -->|成功| C[生成Token]
    B -->|失败| D[返回错误]
    C --> E[返回成功响应]
```

请确保图表语法正确，能够正常渲染。
"#
}
