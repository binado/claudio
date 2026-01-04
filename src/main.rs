use anyhow::Result;
use clap::Parser;
use std::process::ExitCode;

use claudio::cli::Cli;
use claudio::cli::Commands;

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let exit_code = match &cli.command {
        Commands::Run {
            preset,
            claude_args,
        } => claudio::commands::run::run(preset, claude_args)?,
        Commands::List { verbose } => {
            claudio::commands::list::list(*verbose)?;
            ExitCode::SUCCESS
        }
        Commands::Show {
            preset,
            resolved,
            json,
        } => {
            claudio::commands::show::show(preset, *resolved, *json)?;
            ExitCode::SUCCESS
        }
        Commands::Edit { preset } => {
            claudio::commands::edit::edit(preset)?;
            ExitCode::SUCCESS
        }
        Commands::Which { preset } => {
            claudio::commands::which::which(preset)?;
            ExitCode::SUCCESS
        }
        Commands::Env {
            preset,
            export,
            resolved,
        } => {
            claudio::commands::env::env(preset, *export, *resolved)?;
            ExitCode::SUCCESS
        }
    };

    Ok(exit_code)
}
