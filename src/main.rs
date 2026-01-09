use anyhow::Result;
use clap::Parser;
use std::process::ExitCode;

use claudio::cli::{Cli, Commands, PresetCommands, Scope};
use claudio::color::{ColorConfig, HighlightColor};
use claudio::settings::loader as settings_loader;

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Determine the scope for settings lookup (default to Auto for global settings)
    let settings_scope = match &cli.command {
        Some(Commands::Run(args)) => args.scope.scope,
        Some(Commands::Init { scope }) => scope.scope,
        Some(Commands::Preset { command }) => match command {
            PresetCommands::List { scope, .. }
            | PresetCommands::Show { scope, .. }
            | PresetCommands::Edit { scope, .. }
            | PresetCommands::Add { scope, .. }
            | PresetCommands::Env { scope, .. } => scope.scope,
        },
        None => {
            // For shorthand syntax, use the run_args scope
            cli.run_args.scope.scope
        }
    };

    // Initialize color config with settings as defaults, CLI flags override
    let settings_color = match settings_loader::resolve_color(settings_scope) {
        Ok(color) => color,
        Err(e) => {
            eprintln!("Warning: failed to load color setting: {:#}", e);
            None
        }
    };
    let settings_no_color = match settings_loader::resolve_no_color(settings_scope) {
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
            execute_run_command(args, &color_config)?
        }
        Some(Commands::Init { scope }) => {
            claudio::commands::init::init(scope.scope)?;
            ExitCode::SUCCESS
        }
        Some(Commands::Preset { command }) => match command {
            PresetCommands::List {
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
                claudio::commands::show::show(preset, scope.scope, *resolved, *json)?;
                ExitCode::SUCCESS
            }
            PresetCommands::Edit { preset, scope } => {
                claudio::commands::edit::edit(preset, scope.scope)?;
                ExitCode::SUCCESS
            }
            PresetCommands::Add { preset, scope } => {
                claudio::commands::add::add(preset, scope.scope)?;
                ExitCode::SUCCESS
            }
            PresetCommands::Env {
                preset,
                scope,
                export,
                resolved,
            } => {
                claudio::commands::env::env(preset, scope.scope, *export, *resolved)?;
                ExitCode::SUCCESS
            }
        },
        None => {
            // Shorthand: claudio my-preset
            execute_run_command(&cli.run_args, &color_config)?
        }
    };

    Ok(exit_code)
}

fn execute_run_command(
    args: &claudio::cli::RunArgs,
    color_config: &ColorConfig,
) -> Result<ExitCode> {
    // Check for ignore_user_presets conflict with --scope user
    if args.scope.scope == Scope::User && settings_loader::should_ignore_user_presets()? {
        anyhow::bail!(
            "Cannot use --scope user: project settings have 'ignore_user_presets' enabled.\n\n\
             Remove the --scope flag or update your project settings."
        );
    }

    claudio::commands::run::run(
        args.preset.as_deref(),
        args.scope.scope,
        args.require_preset,
        &args.claude_args,
        color_config,
    )
}
