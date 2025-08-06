mod commands;
mod config;
mod git;
mod history;
mod language;
mod llm;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = commands::Cli::parse();

    match cli.command {
        commands::Commands::Init => commands::init::handle_init().await?,
        commands::Commands::Lint { format, ai_enhance } => {
            commands::linter::handle_linter(format, ai_enhance).await?;
        }
        commands::Commands::Commit {
            all,
            lint,
            structured,
            no_edit,
        } => commands::commit::handle_commit(all, lint, structured, no_edit).await?,
        commands::Commands::Report {
            since,
            until,
            period,
        } => commands::report::handler_report(since, until, period).await?,
        commands::Commands::Review { lint } => {
            commands::review::handle_review(lint).await?
        }
        commands::Commands::Archive => commands::archive::handle_archive().await?,
        commands::Commands::InstallHook => {
            commands::install_hook::install_post_commit_hook().await?
        }
        commands::Commands::Branch {
            description,
            create,
            from_staged,
        } => commands::branch::handle_branch(description, create, from_staged).await?,
    }

    Ok(())
}
