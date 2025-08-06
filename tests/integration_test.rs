// tests/integration_test.rs

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::{tempdir, TempDir};

// --- Test Setup Helper ---

struct TestRepo {
    temp_dir: TempDir,
    matecode_path: PathBuf,
}

impl TestRepo {
    fn new() -> Self {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let matecode_path = assert_cmd::cargo::cargo_bin("matecode");
        Self { temp_dir, matecode_path }
    }
    
    fn with_git(self) -> Self {
        git_init(self.temp_dir.path());
        self
    }
    
    fn with_config(self, mock_server_url: &str) -> Self {
        let mut init_cmd = self.matecode();
        init_cmd.arg("init").assert().success();

        let config_path = self.temp_dir.path().join(".config").join("matecode").join("config.toml");

        let test_config_content = format!(r#"
            provider = "openai"
            language = "en-US"

            [llm.openai]
            api_key = "test-key"
            api_base = "{}"
            default_model = "gpt-3.5-turbo"
            models = {{ "gpt-3.5-turbo" = {{ max_tokens = 4096, max_output_tokens = 1024, reserved_tokens = 500 }} }}

            [lint]
            rust = "cargo clippy"
        "#, mock_server_url);
        
        fs::write(config_path, test_config_content)
            .expect("Failed to write test-specific config.toml");
        
        self
    }

    fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    fn matecode(&self) -> Command {
        let mut cmd = Command::new(&self.matecode_path);
        cmd.current_dir(self.path());
        cmd.env("HOME", self.path());
        cmd.env("USERPROFILE", self.path());
        cmd.env("XDG_CONFIG_HOME", self.path().join(".config"));
        cmd
    }
}

fn run_git_command(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .expect(&format!("Failed to execute git command: {:?}", args));
    assert!(output.status.success(), "Git command failed: {:?}, stderr: {}", args, String::from_utf8_lossy(&output.stderr));
}

fn git_init(dir: &Path) {
    run_git_command(dir, &["init"]);
    run_git_command(dir, &["config", "user.name", "Test User"]);
    run_git_command(dir, &["config", "user.email", "test@example.com"]);
}

fn create_and_stage_file(repo_path: &Path, file_name: &str, content: &str) {
    let file_path = repo_path.join(file_name);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).expect("Failed to create parent dir for file");
    }
    fs::write(&file_path, content).expect("Failed to write file");
    run_git_command(repo_path, &["add", file_name]);
}

fn mock_openai_api(server: &mut mockito::Server, mock_response_content: &str) -> mockito::Mock {
    server.mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(r#"{{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": "gpt-3.5-turbo-0125",
            "choices": [{{
                "index": 0,
                "message": {{
                    "role": "assistant",
                    "content": "{}"
                }},
                "finish_reason": "stop"
            }}],
            "usage": {{
                "prompt_tokens": 9,
                "completion_tokens": 12,
                "total_tokens": 21
            }}
        }}"#, mock_response_content))
        .create()
}


// --- Tests ---

#[test]
fn test_init_command() {
    let repo = TestRepo::new();
    let mut cmd = repo.matecode();
    cmd.arg("init");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("✅ 已创建默认配置文件:"));
}

#[tokio::test]
async fn test_branch_command() {
    let mut server = mockito::Server::new_async().await;
    let mock = mock_openai_api(&mut server, "<branch_name>feat/new-awesome-feature</branch_name>");

    let repo = TestRepo::new().with_git().with_config(&server.url());
    let mut cmd = repo.matecode();
    cmd.args(["branch", "a new feature"]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("feat/new-awesome-feature"));
    
    mock.assert();
}

#[tokio::test]
async fn test_commit_command_with_staged_files() {
    let mut server = mockito::Server::new_async().await;
    let mock = mock_openai_api(&mut server, "<commit_message>feat: add new file</commit_message>");

    let repo = TestRepo::new().with_git().with_config(&server.url());
    create_and_stage_file(repo.path(), "file.txt", "initial content\n");

    let mut cmd = repo.matecode();
    cmd.args(["commit", "--no-edit"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("🚀 提交成功！"));

    mock.assert();
}

#[tokio::test]
async fn test_review_command() {
    let mut server = mockito::Server::new_async().await;
    let mock = mock_openai_api(&mut server, "### ✨ LGTM! Looks good to me!");

    let repo = TestRepo::new().with_git().with_config(&server.url());
    create_and_stage_file(repo.path(), "file.txt", "some changes to review\n");

    let mut cmd = repo.matecode();
    cmd.arg("review");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("LGTM!"));
    
    mock.assert();
}

#[test]
fn test_lint_command_plain() {
    let repo = TestRepo::new().with_git().with_config("http://localhost:1234"); // Doesn't call API
    create_and_stage_file(repo.path(), "src/main.rs", "fn main() { let x = 5; }");

    let mut cmd = repo.matecode();
    cmd.arg("lint");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("正在运行命令: cargo clippy"));
}

#[tokio::test]
#[ignore]
async fn test_lint_command_sarif_ai_enhanced() {
    let mut server = mockito::Server::new_async().await;
    let mock_sarif_run = r#"{ \"tool\": { \"driver\": { \"name\": \"matecode AI Review\", \"information_uri\": \"https://github.com/liuwwang/matecode\", \"rules\": [ { \"id\": \"MATE-AI-001\", \"name\": \"AI General Suggestion\", \"short_description\": { \"text\": \"AI-powered analysis\" }, \"full_description\": { \"text\": \"An AI has reviewed the linter output and provided holistic feedback.\"}, \"default_configuration\": { \"level\": \"note\" } } ] } }, \"results\": [ { \"ruleId\": \"MATE-AI-001\", \"message\": { \"text\": \"This is a great starting point. Consider refactoring complex functions.\"}, \"locations\": [] } ] }"#;
    let mock = mock_openai_api(&mut server, mock_sarif_run);

    let repo = TestRepo::new().with_git().with_config(&server.url());
    create_and_stage_file(repo.path(), "src/lib.rs", "pub fn a() -> bool { if true { return true } else { return false } }");

    let mut cmd = repo.matecode();
    cmd.args(["lint", "--format", "--ai-enhance"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains(r#""name": "matecode AI Review""#));
    
    mock.assert();
}

#[tokio::test]
async fn test_report_command() {
    let mut server = mockito::Server::new_async().await;
    let mock = mock_openai_api(&mut server, "### ✨ New Features\\n- Great work on the new report command!");

    let repo = TestRepo::new().with_git().with_config(&server.url());
    create_and_stage_file(repo.path(), "file1.txt", "first commit");
    run_git_command(repo.path(), &["commit", "-m", "feat: initial commit"]);
    
    // The archive command does not call the LLM, so no mock needed here.
    let mut archive_cmd = repo.matecode();
    archive_cmd.arg("archive").assert().success();

    let mut cmd = repo.matecode();
    cmd.arg("report");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Great work on the new report command!"));

    mock.assert();
}
