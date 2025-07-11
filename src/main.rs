//! src/main.rs

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use matecode::{
    cli::{Cli, Commands},
    config, git, llm,
};

async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Commit { .. } => {
            let diff = git::get_staged_diff()?;

            if diff.is_empty() {
                println!("{}", "No staged changes found.".yellow());
                return Ok(());
            }

            let client = config::get_llm_client()?;

            let spinner = ProgressBar::new_spinner();
            spinner.set_style(
                ProgressStyle::default_spinner()
                    .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                    .template("{spinner:.blue} {msg}")?,
            );
            spinner.set_message("Generating commit message...");
            spinner.enable_steady_tick(Duration::from_millis(100));

            let message = llm::generate_commit_message(&client, &diff).await?;

            spinner.finish_and_clear();

            println!("{}", message);
        }
        Commands::Report { .. } => {
            println!("{}", "Report command is not yet implemented.".yellow());
        }
        Commands::Init => {
            let config_path = config::create_default_config()
                .await
                .expect("Failed to create default config");
            println!(
                "{}{}{}",
                "Configuration initialized successfully in ".green(),
                config_path.to_str().unwrap().green(),
                "/".green()
            );
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{}: {:?}", "Error".red(), e);
        std::process::exit(1);
    }
}
