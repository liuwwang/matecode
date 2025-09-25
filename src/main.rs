mod commands;
mod config;
mod git;
mod history;
mod llm;

use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = commands::Cli::parse();

    match cli.command {
        commands::Commands::Init => commands::init::handle_init().await?,
        commands::Commands::Commit {
            all,
            structured,
            no_edit,
        } => commands::commit::handle_commit(all, structured, no_edit).await?,
        commands::Commands::Report {
            since,
            until,
            period,
        } => commands::report::handler_report(since, until, period).await?,
        commands::Commands::Archive => commands::archive::handle_archive().await?,
        commands::Commands::InstallHook => {
            commands::install_hook::install_post_commit_hook().await?
        }
        commands::Commands::Understand { dir } => {
            commands::understand::handle_understand(dir).await?
        }
    }

    Ok(())
}
