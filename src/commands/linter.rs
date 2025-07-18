use colored::Colorize;
use lazy_static::lazy_static;
use regex::Regex;

use crate::config;
use crate::language;
use anyhow::{Result, anyhow};
use std::fmt;
use std::path::PathBuf;
use std::process::Command; // Import Result

#[derive(Debug, Clone)]
pub struct LinterCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl LinterCommand {
    pub fn new(program: String, args: Vec<String>) -> Self {
        LinterCommand { program, args }
    }

    pub fn execute(&self) -> Result<String> {
        let output = Command::new(&self.program)
            .args(&self.args)
            .output()
            .map_err(|e| anyhow!("Failed to spawn command '{}': {}", self, e))?;

        // Combine stdout and stderr. This is crucial for linters that write to stderr.
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Return the combined output. The caller will decide if it's an error.
        Ok(format!("{stdout}{stderr}").trim().to_string())
    }
}

impl fmt::Display for LinterCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.program, self.args.join(" "))
    }
}

pub async fn get_linter_command(
    lang: &str,
    config: &crate::config::Config,
) -> Result<Option<LinterCommand>> {
    // 1. Check user-defined linters in config.toml
    if let Some(command_str) = config.lint.get(lang) {
        if !command_str.starts_with('#') {
            let parts: Vec<&str> = command_str.split_whitespace().collect();
            if let Some(program) = parts.first() {
                let args = parts[1..].iter().map(|&s| s.to_string()).collect();
                return Ok(Some(LinterCommand::new(program.to_string(), args)));
            }
        }
    }

    // 2. Check for project-local linters (e.g., node_modules/.bin/eslint)
    if let Some(linter) = find_project_local_linter(lang).await? {
        return Ok(Some(linter));
    }

    // 3. Check for native linters (e.g., cargo clippy, go vet)
    if let Some(linter) = find_native_linter(lang).await? {
        return Ok(Some(linter));
    }

    // 4. Check for matecode-managed linters (future use)
    if let Some(linter) = find_matecode_managed_linter(lang).await? {
        return Ok(Some(linter));
    }

    Ok(None)
}

async fn find_project_local_linter(lang: &str) -> Result<Option<LinterCommand>> {
    match lang {
        "javascript" | "typescript" => {
            let eslint_path = PathBuf::from("./node_modules/.bin/eslint");
            if tokio::fs::metadata(&eslint_path).await.is_ok() {
                println!("ℹ️  检测到项目本地的 ESLint。");
                return Ok(Some(LinterCommand::new(
                    eslint_path.to_str().unwrap().to_string(),
                    vec![".".to_string()],
                )));
            }
        }
        _ => {}
    }
    Ok(None)
}

async fn find_native_linter(lang: &str) -> Result<Option<LinterCommand>> {
    match lang {
        "rust" => {
            if is_command_in_path("cargo") {
                return Ok(Some(LinterCommand::new(
                    "cargo".to_string(),
                    vec![
                        "clippy".to_string(),
                        "--".to_string(),
                        "-D".to_string(),
                        "warnings".to_string(),
                    ],
                )));
            }
        }
        "go" => {
            if is_command_in_path("go") {
                return Ok(Some(LinterCommand::new(
                    "go".to_string(),
                    vec!["vet".to_string(), "./...".to_string()],
                )));
            }
        }
        "python" => {
            if is_command_in_path("ruff") {
                return Ok(Some(LinterCommand::new(
                    "ruff".to_string(),
                    vec!["check".to_string(), ".".to_string()],
                )));
            }
        }
        _ => {}
    }
    Ok(None)
}

async fn find_matecode_managed_linter(_lang: &str) -> Result<Option<LinterCommand>> {
    // This function is now a placeholder.
    // We could use it in the future to manage tools downloaded by matecode.
    Ok(None)
}

fn is_command_in_path(command: &str) -> bool {
    which::which(command).is_ok()
}

pub async fn handle_linter(show_details: bool) -> Result<Option<String>> {
    let config = config::load_config().await?;

    let lang = match language::detect_project_language()? {
        Some(l) => l,
        None => {
            println!("{}", "🤔 未能检测到项目中的主要编程语言。".yellow());
            return Ok(None);
        }
    };

    if !show_details {
        println!("🔍 正在对 {} 项目进行代码质量检查...", lang.cyan());
    } else {
        println!("🔍 检测到项目语言: {}", lang.cyan());
    }

    let Some(linter_cmd) = get_linter_command(&lang, &config).await? else {
        println!(
            "🤷‍ 未在配置中找到语言 '{}' 对应的 linter 命令。",
            lang.yellow()
        );
        println!("   您可以在 `config.toml` 的 `[lint]` 部分为它添加一个，例如：");
        println!("   {} = \"<your-linter-command>\"", lang);
        return Ok(None);
    };

    if show_details {
        println!("🚀 正在运行命令: {}", linter_cmd.to_string().green());
        println!("{}", "-".repeat(60));
    }

    let full_output = match linter_cmd.execute() {
        Ok(output) => output,
        Err(e) => {
            // This error is now only for when the command fails to spawn.
            eprintln!("{}", format!("❌ Linter 命令执行失败: {}", e).red());
            return Ok(None);
        }
    };

    if full_output.is_empty() {
        println!("{}", "✅ Lint 检查通过，没有发现问题。".green());
        return Ok(Some(full_output));
    }

    if show_details {
        println!("{}", full_output);
        println!("{}", "-".repeat(60));
    }

    let has_errors = full_output.contains("error: could not compile");
    if has_errors {
        if let Some(count) = parse_linter_summary(&full_output) {
            println!(
                "{}",
                format!("❌ Lint 检查发现 {} 个问题。", count).yellow()
            );
        } else {
            println!("{}", "❌ Lint 检查发现问题。".yellow());
        }
    } else {
        println!("{}", "✅ Lint 检查通过，没有发现问题。".green());
    }

    if has_errors && !show_details {
        println!("   请运行 `matecode lint --details` 查看详细信息。");
    }

    Ok(Some(full_output))
}

/// Parses the summary of linter output to find the number of problems.
pub fn parse_linter_summary(output: &str) -> Option<usize> {
    lazy_static! {
        static ref RE: Regex = Regex::new(
            r"(?i)(?:found|aborted due to|generated)\s+(\d+)\s+(?:problems?|errors?|warnings?)"
        )
        .unwrap();
    }

    for line in output.lines().rev().take(5) {
        if let Some(caps) = RE.captures(line) {
            if let Some(count_match) = caps.get(1) {
                if let Ok(count) = count_match.as_str().parse::<usize>() {
                    return Some(count);
                }
            }
        }
    }

    None
}
