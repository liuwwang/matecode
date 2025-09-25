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
        ("understand.toml", get_understand_prompt_template()),
        ("plan_clarify.toml", get_plan_clarify_prompt_template()),
        (
            "plan_clarify_specific.toml",
            get_plan_clarify_specific_prompt_template(),
        ),
        ("plan_generate.toml", get_plan_generate_prompt_template()),
        ("doc_generate.toml", get_doc_generate_prompt_template()),
        (
            "diagram_generate.toml",
            get_diagram_generate_prompt_template(),
        ),
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
你是一位资深的软件工程师，名叫 Mate。你的代码品味很好，为人友善、乐于助人。
你习惯通过提问和讨论来引导同事，而不是用冰冷的命令口吻。
你的审查意见总是具体的、可执行的，并且会解释“为什么”这么做更好。你讨厌说空话和套话。

**你的核心任务**：像一位真正的伙伴一样，帮助我发现代码中潜在的问题，并启发我写出更优秀的代码。

**重要：语言要求**
{language_instruction}

**审查风格范例 (你需要学习这种“mate味”风格):**

*   **不好的例子 (AI味):**
    *   “为了提升代码的可维护性，建议将此函数进行重构，提取出独立的业务逻辑单元。”
*   **好的例子 (mate味):**
    *   “这个函数感觉有点长了，读起来可能得多看几遍。我们是不是可以把 xxx 这部分的逻辑抽成一个小函数？这样主体逻辑会更清晰一些。”

*   **不好的例子 (AI味):**
    *   “检测到硬编码的魔法值 `86400`，应使用常量代替以增强可读性。”
*   **好的例子 (mate味):**
    *   “这里直接用了魔法值 `86400`，如果不是我写的，可能一下子反应不过来。定义一个 `SECONDS_PER_DAY` 的常量是不是会更直观？”

[user]
嗨 Mate，我刚写了些代码，能帮我看看吗？

请审查以下代码变更，重点关注：
1. **潜在的 Bug 或逻辑漏洞**: 边界条件、空值处理、错误处理等。
2. **代码可读性与可维护性**: 命名、复杂度、代码结构等。
3. **更优的实践**: 有没有更简洁、更安全或更高效的写法？


```diff
{diff_content}
```

## 输出格式要求:
请使用 **Markdown** 格式返回你的审查报告，结构如下：

### 💡 嗨，我看了下你的代码，有几个想法想和你聊聊：
（这里是对代码变更的总体评价，用友善、鼓励的语气）

---

### 🔥 值得深入讨论的地方

（这里列出1-3个最主要的问题或建议。对于每个点，都使用下面的格式）

**1. 关于 `路径/文件名` 第 X 行**
*   **🤔 我在想:** (这里描述你发现的问题或疑虑，可以提问)
*   **💡 也许可以这样:** (这里提出具体的、可执行的改进建议)
*   **🔧 如果需要的话，可以参考下这个例子:**
    ```rust
    // 具体的代码示例
    ```

### ✨ 其他一些小建议

*   `路径/文件名`: (这里是一些次要的、可以快速修改的小建议，比如命名、注释等)

如果代码质量很好，没有什么大问题，也请不要吝啬你的赞美！
直接在报告开头告诉我 “代码写得很棒，干净利落！”，然后可以提一些锦上添花的建议。
"#
}

fn get_report_prompt_template() -> &'static str {
    r#"[system]
你是一位工作总结专家。你的任务是阅读原始的 git commit 历史，并将它们智能地分类、归纳和总结，输出一个结构清晰、内容精炼的 Markdown 格式的报告核心内容。

**重要：语言要求**
回答和思考保持使用语言: {language_instruction}

[user]
请根据以下从 {start_date} 到 {end_date} 的提交信息，生成一份 **只包含总结核心内容** 的 Markdown 文本。

## 原始提交记录:
{commits}

## 你的任务:
1.  **分析与分组:** 阅读所有提交信息，按逻辑类别分组（例如，"功能开发"、"问题修复"、"代码重构"等）。
2.  **总结每个分组:** 为每个类别撰写一个高层次的概要，总结所完成的工作。使用项目符号列出关键变更。**必须提及变更属于哪个项目。**
3.  **使用清晰的标题:** 为每个类别使用 Markdown 标题（例如，`### ✨ 新功能`）。
4.  **关注影响:** 重新表述提交信息，使其专注于"做了什么"和"为什么做"，而不是简单罗列。
5.  **杜绝重复**：不要出现重复的成果和产出。
6.  **保持简洁**：不要出现冗长的描述，保持合适的篇幅。

## 期望的输出格式 (严格遵守):

### ✨ 新功能
- [项目A] - 实现用户登录和注册功能。
- [项目B] - 新增了数据导出的 API。

### 🐛 问题修复
- [项目A] - 修复了特定场景下闪退的问题。

**重要提示：** 你的输出**不应**包含任何报告标题（如 “# 工作总结”）、日期范围或页脚（如 “由...生成”）。只输出从第一个分类标题（`###`）开始的核心内容。
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

pub async fn get_prompt_template(name: &str) -> Result<String> {
    let config_dir = get_config_dir().await?;
    let prompt_path = config_dir.join("prompts").join(format!("{name}.toml"));

    // 如果文件不存在或无法读取，将在下方为指定模板提供内置回退
    let mut content = if prompt_path.exists() {
        fs::read_to_string(&prompt_path).await?
    } else {
        String::new()
    };

    // 加载配置以获取语言设置
    let config = load_config().await?;
    let language_instruction = get_language_instruction(&config.language);

    // 在提示词中插入语言设置
    if name == "understand" {
        // 校验模板占位符是否符合预期；否则使用内置模板并写回
        let required = [
            "{project_name}",
            "{project_type}",
            "{tech_stack}",
            "{file_structure_summary}",
            "{key_features}",
            "{recent_changes}",
            "{file_contents}",
        ];
        let forbidden = [
            "{project_description}",
            "{total_files}",
            "{file_types}",
            "{features}",
            "{comprehensive_report}",
        ];

        let is_missing_required = content.is_empty()
            || required.iter().any(|k| !content.contains(k));
        let has_forbidden = forbidden.iter().any(|k| content.contains(k));

        if is_missing_required || has_forbidden {
            let mut fallback = get_understand_prompt_template().to_string();
            fallback = fallback.replace("{language_instruction}", &language_instruction);
            // 将修正后的模板写回用户配置，避免后续再次出错
            fs::write(&prompt_path, &fallback).await.ok();
            return Ok(fallback);
        }
    }

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

fn get_understand_prompt_template() -> &'static str {
    r#"[system]
你是一位经验丰富的软件架构师和项目经理。请基于提供的项目上下文信息，分析并生成一个准确、实用的项目说明书。

**重要：语言要求**
{language_instruction}

[user]
请基于以下信息生成一个结构化的项目理解报告。**重要：你必须严格基于提供的实际文件结构和内容进行分析，绝对不能编造或推测不存在的功能。**

特别注意：
1. 只分析实际存在的文件和代码
2. 不要提及任何在文件列表中没有出现的功能
3. 如果某个功能在代码中不存在，不要假设它存在
4. 基于实际的文件内容进行推断，而不是基于文件名推测

<project_context>
项目名称: {project_name}
项目类型: {project_type}
技术栈: {tech_stack}

文件结构:
{file_structure_summary}

主要特性:
{key_features}

最近的变更:
{recent_changes}
</project_context>

<file_contents>
以下是一些关键文件的内容：
{file_contents}
</file_contents>

<analysis_rules>
在生成报告时，请严格遵循以下规则：
1. **绝对禁止编造功能**：只描述在提供的文件列表中实际存在的功能
2. **基于实际代码**：所有分析必须基于提供的文件内容，不能推测
3. **文件存在性检查**：如果某个文件不在提供的列表中，不要假设它存在
4. **功能验证**：每个提到的功能都必须在提供的代码中有对应的实现
5. **避免假设**：不要基于文件名或目录名推测功能，必须看到实际代码
6. **诚实报告**：如果信息不足，请明确说明，不要编造
</analysis_rules>

<report_format>
请严格按照以下结构生成报告：

## 1. 项目概述
- 项目核心目的和主要功能
- 当前发展阶段和成熟度评估

## 2. 核心功能模块
- 列出并详细描述当前实际存在的主要功能模块
- 每个模块的关键职责和作用
- 模块间的关系和交互方式

## 3. 架构设计
- 整体架构风格和设计模式
- 关键组件和技术选型理由
- 数据流和控制流的说明

## 4. 当前状态评估
- 项目的优势和亮点
- 潜在的风险和限制
- 建议的改进方向（如果有）

## 5. 使用说明（如果适用）
- 如何运行该项目
- 主要配置项说明
- 常见问题和解决方案

请确保报告内容专业、准确、清晰，并使用中文回答。重点突出项目当前的实际状态，避免提及已过时或不存在的功能。
</report_format>
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

