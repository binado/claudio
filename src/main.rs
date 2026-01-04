use clap::Parser;
use std::process;

use claudio::cli::Cli;
use claudio::cli::Commands;

fn main() {
    let cli = Cli::parse();

    let exit_code = match &cli.command {
        Commands::Run {
            preset,
            claude_args,
        } => match claudio::commands::run::run(preset, claude_args) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Error: {}", e);
                1
            }
        },
        Commands::List { verbose } => match claudio::commands::list::list(*verbose) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("Error: {}", e);
                1
            }
        },
        Commands::Show { preset, resolved } => {
            match claudio::commands::show::show(preset, *resolved) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    1
                }
            }
        }
        Commands::Edit { preset } => match claudio::commands::edit::edit(preset) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("Error: {}", e);
                1
            }
        },
        Commands::Which { preset } => match claudio::commands::which::which(preset) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("Error: {}", e);
                1
            }
        },
        Commands::Env {
            preset,
            export,
            resolved,
        } => match claudio::commands::env::env(preset, *export, *resolved) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("Error: {}", e);
                1
            }
        },
    };

    process::exit(exit_code)
}
