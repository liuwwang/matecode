//! src/git.rs
use std::process::{Command, Output};
use anyhow::Result;
use ignore::gitignore::GitignoreBuilder;
use std::path::Path;
use crate::config::get_config_dir;

fn run_git_command(args: &[&str]) -> Result<Output> {
    // 跨平台的 Git 命令调用
    let git_cmd = if cfg!(windows) {
        // Windows 上优先尝试 git.exe
        "git.exe"
    } else {
        "git"
    };
    
    let output = Command::new(git_cmd)
        .args(args)
        .output()
        .or_else(|_| {
            // 如果失败，尝试另一种方式
            let fallback_cmd = if cfg!(windows) { "git" } else { "git.exe" };
            Command::new(fallback_cmd).args(args).output()
        })?;
    
    if output.status.success() {
        Ok(output)
    } else {
        Err(anyhow::anyhow!(
            "Git 命令执行失败 (退出码: {}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// 获取暂存区（staged）的代码变更，并应用 .gitignore 和 .matecode-ignore 的规则
pub fn get_staged_diff() -> Result<String> {
    // Determine the reference to compare against.
    // If HEAD exists, use it. Otherwise, assume initial commit and use the empty tree.
    let head_exists = run_git_command(&["rev-parse", "--verify", "HEAD"]).is_ok();
    let parent_ref = if head_exists {
        "HEAD"
    } else {
        // This is the magic number for an empty tree in Git, used for the initial commit.
        "4b825dc642cb6eb9a060e54bf8d69288fbee4904"
    };

    // 1. 获取所有暂存的文件名
    let name_only_output =
        run_git_command(&["diff-index", "--cached", "--name-only", "--no-renames", parent_ref])?;
    let staged_files_raw = String::from_utf8_lossy(&name_only_output.stdout);
    if staged_files_raw.trim().is_empty() {
        return Ok(String::new());
    }

    // 2. 构建一个 Gitignore 匹配器
    let mut builder = GitignoreBuilder::new(".");

    // 添加用户自定义的忽略文件（从家目录的配置文件夹读取）
    if let Ok(config_dir) = get_config_dir() {
        let matecode_ignore_path = config_dir.join(".matecode-ignore");
        if matecode_ignore_path.exists() {
            builder.add(matecode_ignore_path);
        }
    }

    // 添加硬编码的、内置的忽略规则
    // todo: 后面可以考虑将这部分也做成可配置的
    builder.add_line(None, "*.json")?;

    let ignorer = builder.build()?;

    // 3. 在内存中筛选需要 diff 的文件
    let mut files_to_diff = Vec::new();
    for file_path_str in staged_files_raw.lines() {
        let file_path = Path::new(file_path_str);

        let is_hidden = file_path
            .components()
            .any(|c| c.as_os_str().to_string_lossy().starts_with('.'));
        if is_hidden {
            println!("ℹ️  已忽略隐藏文件/目录: {}", file_path_str);
            continue;
        }

        let is_ignored = ignorer.matched(file_path, false).is_ignore();

        if !is_ignored {
            files_to_diff.push(file_path_str);
        } else {
            println!(
                "ℹ️  已根据 .gitignore/.matecode-ignore 忽略文件: {}",
                file_path_str
            );
        }
    }

    // 4. 如果没有剩下任何文件，则返回空字符串
    if files_to_diff.is_empty() {
        return Ok(String::new());
    }

    // 5. 一次性调用 git diff 获取所有未被忽略的文件的变更
    let mut command_args = vec![
        "diff-index",
        "--patch",
        "--cached",
        "--no-renames",
        parent_ref,
        "--",
    ];
    command_args.extend(files_to_diff);

    let diff_output = run_git_command(&command_args)?;
    let diff_string = String::from_utf8_lossy(&diff_output.stdout).to_string();

    Ok(diff_string)
} 