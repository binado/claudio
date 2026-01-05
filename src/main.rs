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
            scope,
            claude_args,
        } => claudio::commands::run::run(preset.as_deref(), scope.scope, claude_args)?,
        Commands::List {
            scope,
            name,
            verbose,
            fields,
            prompt_max_length,
        } => {
            claudio::commands::list::list(
                scope.scope,
                name.as_deref(),
                *verbose,
                fields.as_deref(),
                *prompt_max_length,
            )?;
            ExitCode::SUCCESS
        }
        Commands::Show {
            preset,
            scope,
            resolved,
            json,
        } => {
            claudio::commands::show::show(preset, scope.scope, *resolved, *json)?;
            ExitCode::SUCCESS
        }
        Commands::Edit { preset, scope } => {
            claudio::commands::edit::edit(preset, scope.scope)?;
            ExitCode::SUCCESS
        }
        Commands::Init { scope } => {
            claudio::commands::init::init(scope.scope)?;
            ExitCode::SUCCESS
        }
        Commands::Env {
            preset,
            scope,
            export,
            resolved,
        } => {
            claudio::commands::env::env(preset, scope.scope, *export, *resolved)?;
            ExitCode::SUCCESS
        }
    };

    Ok(exit_code)
}
