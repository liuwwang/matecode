//! src/commands/linter.rs

use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use indicatif::ProgressBar;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::process::{Command, Output};

use crate::config::{self, get_llm_client};
use crate::language;
use crate::llm::{parse_prompt_template, LLMClient};

// --- Command Struct ---
#[derive(Debug, Clone)]
pub struct LinterCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl LinterCommand {
    pub fn new(program: String, args: Vec<String>) -> Self {
        Self { program, args }
    }

    pub fn execute(&self) -> Result<Output> {
        Command::new(&self.program)
            .args(&self.args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .map_err(|e| anyhow!("Failed to spawn command '{}': {}", self, e))
    }
}

impl fmt::Display for LinterCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.program, self.args.join(" "))
    }
}

// --- Linter Discovery ---
pub async fn get_linter_command(
    lang: &str,
    config: &config::Config,
    force_json: bool,
) -> Result<Option<LinterCommand>> {
    if let Some(command_str) = config.lint.get(lang) {
        if !command_str.starts_with('#') {
            let parts: Vec<&str> = command_str.split_whitespace().collect();
            if let Some(program) = parts.first() {
                let mut args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

                // Special handling for rust to ensure JSON output if needed,
                // as it's not a standard SARIF output.
                if lang == "rust" && force_json && !command_str.contains("--message-format=json") {
                    if let Some(pos) = args.iter().position(|arg| arg == "--") {
                        // Insert before the `--` separator
                        args.insert(pos, "--message-format=json".to_string());
                    } else {
                        // No separator, just push it
                        args.push("--message-format=json".to_string());
                    }
                }

                return Ok(Some(LinterCommand::new(program.to_string(), args)));
            }
        }
    }
    find_native_linter(lang, force_json).await
}

async fn find_native_linter(lang: &str, force_json: bool) -> Result<Option<LinterCommand>> {
    // This function can be expanded to find more default linters
    if lang == "rust" && is_command_in_path("cargo") {
        let mut args = vec!["clippy".to_string()];
        if force_json {
            args.push("--message-format=json".to_string());
        }
        return Ok(Some(LinterCommand::new("cargo".to_string(), args)));
    }
    Ok(None)
}

fn is_command_in_path(command: &str) -> bool {
    which::which(command).is_ok()
}

// --- Main Handler ---
pub async fn handle_linter(
    sarif: bool,
    ai_enhance: bool,
    _file: Option<String>, // Keep signature for now, but mark unused
) -> Result<Option<String>> {
    let config = config::load_config().await?;
    let lang = match language::detect_project_language()? {
        Some(l) => l,
        None => {
            println!("{}", "🤔 未能检测到项目中的主要编程语言。".yellow());
            return Ok(None);
        }
    };

    if sarif {
        handle_sarif_output(&lang, &config, ai_enhance).await?;
        Ok(None)
    } else {
        handle_plain_output(&lang, &config).await
    }
}

// --- Plain Text Output Logic ---
async fn handle_plain_output(lang: &str, config: &config::Config) -> Result<Option<String>> {
    println!("🔍 正在对 {} 项目进行代码质量检查...", lang.cyan());

    let Some(linter_cmd) = get_linter_command(lang, config, false).await? else {
        println!("🤷‍ 未找到语言 '{}' 对应的 linter 命令。", lang.yellow());
        return Ok(None);
    };

    println!("🚀 正在运行命令: {}", linter_cmd.to_string().green());

    let output = linter_cmd.execute()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let combined_output = format!("{stdout}\n{stderr}");

    if !output.status.success() && !combined_output.trim().is_empty() {
        println!("📋 Linter 输出:\n{combined_output}");
        return Ok(Some(combined_output.to_string()));
    }

    if combined_output.trim().is_empty() {
        println!("{}", "✅ Lint 检查通过，没有发现问题。".green());
        return Ok(None);
    }

    println!("📋 Linter 输出:\n{combined_output}");
    Ok(Some(combined_output.to_string()))
}

// --- SARIF Output Logic ---
async fn handle_sarif_output(lang: &str, config: &config::Config, ai_enhance: bool) -> Result<()> {
    println!("🔍 正在生成 SARIF 报告...");

    // We force JSON for Rust, but for others, we assume they might output SARIF directly.
    let force_rust_json = lang == "rust";
    let Some(linter_cmd) = get_linter_command(lang, config, force_rust_json).await? else {
        println!("🤷‍ 未找到语言 '{}' 对应的 linter 命令。", lang.yellow());
        return Ok(());
    };

    println!("🚀 正在运行命令: {}", linter_cmd.to_string().green());

    let output = linter_cmd.execute()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut sarif_report = match parse_linter_output(&stdout) {
        Ok(Some(report)) => report,
        Ok(None) => {
            println!("{}", "ℹ️ Linter 未输出任何可分析的内容。".yellow());
            return Ok(());
        }
        Err(e) => {
            println!(
                "{}{}",
                "🚫 无法解析 Linter 输出: ".red(),
                e.to_string().red()
            );
            println!("Linter raw output:\n{}", stdout);
            return Ok(());
        }
    };

    if ai_enhance {
        println!("🤖 正在使用 AI 进行宏观分析...");
        let llm_client = get_llm_client().await?;
        match analyze_sarif_report(&sarif_report, llm_client.as_client()).await {
            Ok(ai_run) => {
                println!("🤖 AI 分析完成，正在合并结果...");
                sarif_report.runs.push(ai_run);
            }
            Err(e) => {
                println!(
                    "⚠️ AI 分析失败: {}。将仅显示原始 linter 结果。",
                    e.to_string().yellow()
                );
            }
        };
    }

    let pretty_json = serde_json::to_string_pretty(&sarif_report)?;
    println!("{pretty_json}");

    Ok(())
}

fn parse_linter_output(output: &str) -> Result<Option<SarifReport>> {
    if output.trim().is_empty() {
        return Ok(None);
    }

    // Attempt to parse as a full SARIF report first.
    if let Ok(mut report) = serde_json::from_str::<SarifReport>(output) {
        println!("📄 检测到原生 SARIF 输出，直接解析...");
        // Ensure schema and version are set to our standard, as some tools might omit them.
        report.schema =
            "https://schemastore.azurewebsites.net/schemas/json/sarif-2.1.0-rtm.5.json"
                .to_string();
        report.version = "2.1.0".to_string();
        return Ok(Some(report));
    }

    // Fallback: Attempt to parse as line-delimited JSON (like `cargo clippy`).
    let messages: Vec<LinterMessage> = output
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    if !messages.is_empty() {
        println!(
            "📄 检测到 {} 个需转换的 linter 问题，正在生成 SARIF 报告...",
            messages.len()
        );
        return Ok(Some(linter_messages_to_sarif_report(&messages)?));
    }

    Err(anyhow!(
        "输出既不是有效的 SARIF 格式，也不是可识别的行分隔 JSON 消息。"
    ))
}

fn linter_messages_to_sarif_report(messages: &[LinterMessage]) -> Result<SarifReport> {
    let mut results = Vec::new();
    let mut rules = HashMap::new();

    for msg in messages {
        if let Some(diagnostic) = &msg.message {
            let rule_id = diagnostic
                .code
                .as_ref()
                .map_or("unknown".to_string(), |c| c.code.clone());

            if !rules.contains_key(&rule_id) {
                let full_description_text = diagnostic
                    .rendered
                    .as_ref()
                    .map(|s| clean_rendered_text(s)) // Using a closure here
                    .unwrap_or_default();

                rules.insert(
                    rule_id.clone(),
                    SarifRule {
                        id: rule_id.clone(),
                        name: diagnostic.message.clone(),
                        short_description: SarifMessage {
                            text: diagnostic.message.clone(),
                        },
                        full_description: SarifMessage {
                            text: full_description_text,
                        },
                        default_configuration: SarifDefaultConfiguration {
                            level: diagnostic.level.clone(),
                        },
                    },
                );
            }

            if let Some(span) = diagnostic.spans.iter().find(|s| s.is_primary) {
                results.push(SarifResult {
                    rule_id: rule_id.clone(),
                    message: SarifMessage {
                        text: diagnostic.message.clone(),
                    },
                    locations: vec![SarifLocation {
                        physical_location: SarifPhysicalLocation {
                            artifact_location: SarifArtifactLocation {
                                uri: span.file_name.clone(),
                            },
                            region: SarifRegion {
                                start_line: Some(span.line_start),
                                snippet: span
                                    .text
                                    .first()
                                    .map(|t| SarifSnippet { text: t.text.clone() }),
                            },
                        },
                    }],
                });
            }
        }
    }

    Ok(SarifReport {
        schema: "https://schemastore.azurewebsites.net/schemas/json/sarif-2.1.0-rtm.5.json"
            .to_string(),
        version: "2.1.0".to_string(),
        runs: vec![SarifRun {
            tool: SarifTool {
                driver: SarifDriver {
                    name: "rustc-clippy".to_string(),
                    information_uri: "https://github.com/rust-lang/rust-clippy".to_string(),
                    rules: rules.into_values().collect(),
                },
            },
            results,
        }],
    })
}

fn clean_rendered_text(rendered: &str) -> String {
    let stripped_bytes = strip_ansi_escapes::strip(rendered.as_bytes());
    let stripped = String::from_utf8_lossy(&stripped_bytes);

    let lines: Vec<&str> = stripped
        .lines()
        .filter(|line| !line.starts_with("  = help:"))
        .map(|line| line.trim_end())
        .collect();

    let joined = lines.join("\n");
    let mut cleaned = String::new();
    let mut consecutive_newlines = 0;
    for c in joined.chars() {
        if c == '\n' {
            consecutive_newlines += 1;
            if consecutive_newlines <= 2 {
                cleaned.push(c);
            }
        } else {
            consecutive_newlines = 0;
            cleaned.push(c);
        }
    }

    cleaned.trim().to_string()
}

async fn analyze_sarif_report(report: &SarifReport, llm_client: &dyn LLMClient) -> Result<SarifRun> {
    let sarif_content = serde_json::to_string(report)?;
    let template = config::get_prompt_template("review_sarif").await?;
    let (system_prompt, user_prompt) = parse_prompt_template(&template)?;
    let prompt = user_prompt.replace("{sarif_content}", &sarif_content);

    let pb = ProgressBar::new_spinner().with_message("AI is performing a holistic review...");
    pb.enable_steady_tick(std::time::Duration::from_millis(120));

    let ai_response_str = llm_client.call(&system_prompt, &prompt).await?;

    pb.finish_with_message("✓ AI review complete");

    serde_json::from_str(&ai_response_str).with_context(|| {
        format!("Failed to parse AI response into SarifRun. Response:\n{ai_response_str}")
    })
}

// --- Structs ---

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct LinterMessage {
    pub reason: String,
    pub message: Option<Diagnostic>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Diagnostic {
    pub message: String,
    pub code: Option<DiagnosticCode>,
    pub level: String,
    pub spans: Vec<DiagnosticSpan>,
    pub children: Vec<Diagnostic>,
    pub rendered: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DiagnosticCode {
    pub code: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DiagnosticSpan {
    pub file_name: String,
    pub line_start: usize,
    pub is_primary: bool,
    pub text: Vec<DiagnosticSpanText>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DiagnosticSpanText {
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SarifReport {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    pub runs: Vec<SarifRun>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SarifDriver {
    pub name: String,
    pub information_uri: String,
    pub rules: Vec<SarifRule>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SarifRule {
    pub id: String,
    pub name: String,
    pub short_description: SarifMessage,
    pub full_description: SarifMessage,
    pub default_configuration: SarifDefaultConfiguration,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SarifDefaultConfiguration {
    pub level: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SarifResult {
    pub rule_id: String,
    pub message: SarifMessage,
    pub locations: Vec<SarifLocation>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SarifMessage {
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SarifLocation {
    pub physical_location: SarifPhysicalLocation,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SarifPhysicalLocation {
    pub artifact_location: SarifArtifactLocation,
    pub region: SarifRegion,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SarifArtifactLocation {
    pub uri: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SarifRegion {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<SarifSnippet>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SarifSnippet {
    pub text: String,
}
