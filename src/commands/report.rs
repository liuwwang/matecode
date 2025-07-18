use crate::config;
use crate::config::get_prompt_template;
use crate::history;
use crate::llm::LLMClient;
use anyhow::{Context, Result};
use chrono::NaiveDate;
use colored::Colorize;
use std::collections::BTreeMap;

fn format_commits_for_report(commits: &BTreeMap<String, Vec<String>>) -> String {
    let mut report = String::new();
    for (author, messages) in commits {
        report.push_str(&format!("- **{author}**\n"));
        for msg in messages {
            report.push_str(&format!("  - {msg}\n"));
        }
        report.push('\n');
    }
    report
}

async fn generate_report_from_commits(
    client: &dyn LLMClient,
    commits: &BTreeMap<String, Vec<String>>,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<String> {
    let template = get_prompt_template("report").await?;
    let (system_prompt, user_prompt) = crate::llm::parse_prompt_template(&template)?;

    let commits_text = format_commits_for_report(commits);
    let user_prompt = user_prompt
        .replace("{start_date}", &start_date.to_string())
        .replace("{end_date}", &end_date.to_string())
        .replace("{commits}", &commits_text);

    client.call(&system_prompt, &user_prompt).await
}

pub async fn handler_report(since: Option<String>, until: Option<String>) -> Result<()> {
    let now = chrono::Local::now().date_naive();

    let start_date = since
        .and_then(|s| dateparser::parse(&s).ok())
        .map(|dt| dt.date_naive())
        .unwrap_or(now);

    let end_date = until
        .and_then(|s| dateparser::parse(&s).ok())
        .map(|dt| dt.date_naive())
        .unwrap_or(now);

    let all_commits = history::get_all_commits_in_range(start_date, end_date)
        .await
        .context("无法获取用于报告的提交历史。")?;

    if all_commits.is_empty() {
        println!("{}", "在此日期范围内没有找到任何提交记录。".yellow());
        return Ok(());
    }

    let llm_client = config::get_llm_client().await?;
    let report =
        generate_report_from_commits(llm_client.as_client(), &all_commits, start_date, end_date)
            .await?;
    println!("{report}");
    Ok(())
}
