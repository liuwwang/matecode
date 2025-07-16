//! src/toolchain.rs
// Manages discovery and execution of language-specific linters.

use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;

use crate::config::Config;

/// Represents a command to be executed, abstracting away its source.
#[derive(Debug)]
pub struct LinterCommand {
    program: String,
    args: Vec<String>,
}

impl LinterCommand {
    pub fn new(program: &str, args: &[&str]) -> Self {
        LinterCommand {
            program: program.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn to_command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args);
        cmd
    }

    pub fn to_string(&self) -> String {
        format!("{} {}", self.program, self.args.join(" "))
    }
}

/// Gets the appropriate linter command for a given language.
pub async fn get_linter_command(lang: &str, config: &Config) -> Result<Option<LinterCommand>> {
    if let Some(command) = find_project_local_linter(lang).await? {
        return Ok(Some(command));
    }
    if let Some(command) = find_matecode_managed_linter(lang).await? {
        return Ok(Some(command));
    }
    if let Some(command) = find_native_linter(lang).await? {
        return Ok(Some(command));
    }
    if let Some(command_str) = config.lint.get(lang) {
        if !command_str.starts_with('#') {
            let parts: Vec<&str> = command_str.split_whitespace().collect();
            if let Some(program) = parts.get(0) {
                let args = parts.get(1..).unwrap_or(&[]).to_vec();
                return Ok(Some(LinterCommand::new(program, &args)));
            }
        }
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
                    eslint_path.to_str().unwrap(),
                    &["."],
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
                    "cargo",
                    &["clippy", "--", "-D", "warnings"],
                )));
            }
        }
        "go" => {
            if is_command_in_path("go") {
                return Ok(Some(LinterCommand::new("go", &["vet", "./..."])));
            }
        }
        "python" => {
            if is_command_in_path("ruff") {
                return Ok(Some(LinterCommand::new("ruff", &["check", "."])));
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