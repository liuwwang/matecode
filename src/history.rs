//! src/history.rs

use crate::config::get_config_dir;
use crate::llm::{LLMClient, LLM};
use anyhow::Result;
use chrono::Local;
use std::collections::BTreeMap;
use std::fs::{create_dir_all, read_dir, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

fn get_history_dir() -> Result<PathBuf> {
    let config_dir = get_config_dir()?;
    let history_dir = config_dir.join("history");
    if !history_dir.exists() {
        create_dir_all(&history_dir)?;
    }
    Ok(history_dir)
}

pub fn archive_commit_message(project_name: &str, message: &str) -> Result<()> {
    let history_dir = get_history_dir()?;
    let project_history_dir = history_dir.join(project_name);
    if !project_history_dir.exists() {
        create_dir_all(&project_history_dir)?;
    }

    let today = Local::now().format("%Y-%m-%d").to_string();
    let daily_file_path = project_history_dir.join(format!("{}.md", today));

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(daily_file_path)?;

    let timestamp = Local::now().format("%H:%M:%S").to_string();
    let formatted_message = format!("\n---\n\n**{}**\n\n```\n{}\n```\n", timestamp, message);

    file.write_all(formatted_message.as_bytes())?;

    Ok(())
}

/// 用于传递给AI的结构化日报数据
#[derive(Debug)]
pub struct DailyReportData {
    pub date: String,
    pub projects: BTreeMap<String, String>, // 使用 BTreeMap 来保证项目顺序
}

pub fn gather_daily_commits() -> Result<DailyReportData> {
    let history_dir = get_history_dir()?;
    let today = Local::now().format("%Y-%m-%d").to_string();
    let mut projects = BTreeMap::new();

    let project_dirs = match read_dir(history_dir) {
        Ok(dirs) => dirs,
        Err(_) => {
            return Ok(DailyReportData {
                date: today,
                projects,
            })
        } // History dir might be empty
    };

    for project_dir_entry in project_dirs.filter_map(|e| e.ok()) {
        if !project_dir_entry.path().is_dir() {
            continue;
        }

        let project_name = project_dir_entry.file_name().to_string_lossy().to_string();
        let daily_file_path = project_dir_entry.path().join(format!("{}.md", &today));

        if daily_file_path.exists() {
            if let Ok(content) = std::fs::read_to_string(daily_file_path) {
                if !content.trim().is_empty() {
                    projects.insert(project_name, content);
                }
            }
        }
    }

    Ok(DailyReportData {
        date: today,
        projects,
    })
}

pub async fn generate_ai_powered_report(client: &LLM, report_data: &DailyReportData) -> Result<String> {
    if report_data.projects.is_empty() {
        return Ok(format!(
            "# 📅 {} 工作日报\n\n今日暂无已记录的提交。",
            report_data.date
        ));
    }

    let system_prompt = r#"你是一位顶级的技术经理或资深工程师，擅长从零散的Git commit记录中，提炼和总结出核心的工作成果，并编写一份高度概括、重点突出的中文工作日报。"#;

    let mut commits_context = String::new();
    for (project, commits) in &report_data.projects {
        commits_context.push_str(&format!(
            "\n---\n项目: {}\n---\n{}\n",
            project, commits
        ));
    }

    let user_prompt = format!(
        r#"请根据以下我今天在不同项目中的 git commit 记录，为我生成一份高度凝练、重点突出的工作日报。

**日期**: {}

**原始Commit记录**:
{}

**日报生成要求**:
1.  **目标**: 你的任务不是罗列所有工作，而是从所有 commit 中提炼出今天完成的 **核心成果**。将多个相关的 commit 合并成一个成果描述。
2.  **格式**: 必须严格遵循以下格式，使用有序列表：
    `1. 优化[项目名] - 对XX功能的优化，解决了YY问题。`
    `2. 实现[项目名] - 新增了XX模块，用于YY目的。`
    `3. 排查[项目名] - 定位并修复了XX的bug，原因是YY。`
3.  **内容**:
    - **动词开头**: 每个要点都应该以一个概括性的动词开头（例如：优化、实现、排查、支持、重构、测试等）。
    - **项目标签**: 紧跟动词后，用方括号 `[]` 标明项目名称。
    - **成果描述**: 清晰、简洁地描述你完成的工作成果和价值。
4.  **禁止项**:
    - **绝对不要**包含“明日计划”、“未来展望”或任何与今日工作无关的总结性话语。
    - **不要**有开场白或引言，直接开始第一条工作总结。
    - **不要**逐字逐句地复述 commit message。

**优秀日报示例**:
1. 支持[信通院] - 外出去与信通院郭建去国家测评中心开会，收集扫描漏洞，给出修改意见。
2. 优化[ai-platform] - (知识库/AI搜索)插件的最终搜索结果，包装成ToolMessage的形式给模型，让问答过程更合理。
3. 排查[ai-platform] - （AI搜索）bing搜索结果异常的问题，最后确认是bing搜索插件没有关闭自动化控制选项和等待时间太短导致采集时页面还未渲染的情况。
4. 优化[docflow] - (创建任务) 如果遇到任务已经完成的情况，在返回任务已完成的同时发送一条SSE消息给后端。
"#,
        report_data.date, commits_context
    );

    client.call(system_prompt, &user_prompt).await
} 