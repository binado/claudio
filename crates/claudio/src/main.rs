use anyhow::Result;
use clap::Parser;
use std::process::ExitCode;

mod cli;
mod color;
mod commands;

use crate::cli::{Cli, Commands, PresetCommands, Scope};
use crate::color::{ColorConfig, HighlightColor};
use claudio_core::settings::loader as settings_loader;

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Handle completion command immediately, before loading settings
    if let Some(Commands::Completion { shell }) = &cli.command {
        if let Err(e) = crate::commands::completion::completion(*shell) {
            eprintln!("Error: {:#}", e);
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }

    // Determine the scope for settings lookup (default to Auto for global settings)
    let settings_scope = match &cli.command {
        Some(Commands::Run(args)) => args.scope.scope,
        Some(Commands::Init { scope, .. }) => scope.scope,
        Some(Commands::Doctor { scope, .. }) => scope.scope,
        Some(Commands::Preset { command }) => match command {
            PresetCommands::List { scope, .. }
            | PresetCommands::Show { scope, .. }
            | PresetCommands::Edit { scope, .. }
            | PresetCommands::Add { scope, .. }
            | PresetCommands::Env { scope, .. } => scope.scope,
        },
        Some(Commands::Completion { .. }) => {
            unreachable!("Completion handled before reaching here")
        }
        None => {
            // For shorthand syntax, use the run_args scope
            cli.run_args.scope.scope
        }
    };

    // Initialize color config with settings as defaults, CLI flags override
    let settings_color = match settings_loader::resolve_color(settings_scope.into()) {
        Ok(color) => color,
        Err(e) => {
            eprintln!("Warning: failed to load color setting: {:#}", e);
            None
        }
    };
    let settings_no_color = match settings_loader::resolve_no_color(settings_scope.into()) {
        Ok(no_color) => no_color.unwrap_or(false),
        Err(e) => {
            eprintln!("Warning: failed to load no_color setting: {:#}", e);
            false
        }
    };

    // CLI --color flag uses "cyan" as default, so we only use settings if CLI is at default
    let effective_color = if cli.color == "cyan" {
        settings_color.as_deref().unwrap_or("cyan")
    } else {
        &cli.color
    };

    let highlight_color = match HighlightColor::parse(effective_color) {
        Some(color) => color,
        None => {
            eprintln!(
                "Warning: invalid color '{}'; falling back to Cyan.",
                effective_color
            );
            HighlightColor::Cyan
        }
    };

    // CLI --no-color flag overrides settings (if CLI flag is set, use it; otherwise use settings)
    let no_color = cli.no_color || settings_no_color;
    let color_config = ColorConfig::new(highlight_color, !no_color);

    match run(cli, color_config.clone()) {
        Ok(code) => code,
        Err(e) => {
            let error_prefix = color_config.error("Error:");
            eprintln!("{} {:#}", error_prefix, e);
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli, color_config: ColorConfig) -> Result<ExitCode> {
    let exit_code = match &cli.command {
        Some(Commands::Run(args)) => {
            // Explicit: claudio run my-preset
            execute_run_command(args, &color_config, args.claude_executable.as_deref())?
        }
        Some(Commands::Init {
            scope,
            dry_run,
            quiet,
            force,
        }) => {
            crate::commands::init::init(
                scope.scope.into(),
                *dry_run,
                *quiet,
                *force,
                &color_config,
            )?;
            ExitCode::SUCCESS
        }
        Some(Commands::Doctor {
            scope,
            claude_executable,
            json,
            verbose,
        }) => crate::commands::doctor::doctor(
            scope.scope.into(),
            *json,
            *verbose,
            claude_executable.as_deref(),
        )?,
        Some(Commands::Preset { command }) => match command {
            PresetCommands::List {
                scope,
                name,
                fields,
                max_width,
                json,
            } => {
                crate::commands::list::list(
                    scope.scope.into(),
                    name.as_deref(),
                    fields.as_deref(),
                    *max_width,
                    *json,
                    &color_config,
                )?;
                ExitCode::SUCCESS
            }
            PresetCommands::Show {
                preset,
                scope,
                resolved,
                json,
            } => {
                crate::commands::show::show(preset, scope.scope.into(), *resolved, *json)?;
                ExitCode::SUCCESS
            }
            PresetCommands::Edit { preset, scope } => {
                crate::commands::edit::edit(preset, scope.scope.into())?;
                ExitCode::SUCCESS
            }
            PresetCommands::Add {
                preset,
                extends,
                scope,
            } => {
                crate::commands::add::add(preset, extends.clone(), scope.scope.into())?;
                ExitCode::SUCCESS
            }
            PresetCommands::Env {
                preset,
                scope,
                export,
                resolved,
            } => {
                crate::commands::env::env(preset, scope.scope.into(), *export, *resolved)?;
                ExitCode::SUCCESS
            }
        },
        Some(Commands::Completion { .. }) => {
            unreachable!("Completion handled before reaching here")
        }
        None => {
            // Shorthand: claudio my-preset
            execute_run_command(
                &cli.run_args,
                &color_config,
                cli.run_args.claude_executable.as_deref(),
            )?
        }
    };

    Ok(exit_code)
}

fn execute_run_command(
    args: &crate::cli::RunArgs,
    color_config: &ColorConfig,
    claude_executable: Option<&str>,
) -> Result<ExitCode> {
    // Check for ignore_user_presets conflict with --scope user
    if args.scope.scope == Scope::User
        && settings_loader::should_ignore_user_presets().unwrap_or(false)
    {
        anyhow::bail!(
            "Cannot use --scope user: project settings have 'ignore_user_presets' enabled.\n\n\
             Remove the --scope flag or update your project settings."
        );
    }

    crate::commands::run::run(
        args.preset.as_deref(),
        args.scope.scope.into(),
        args.require_preset,
        args.dry_run,
        &args.claude_args,
        claude_executable,
        color_config,
    )
}
