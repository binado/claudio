use crate::cli::Scope;
use crate::preset::loader;
use crate::preset::resolver;
use anyhow::{Context, Result};
use std::process::{Command, ExitCode};

pub fn run(preset_name: Option<&str>, scope: Scope, claude_args: &[String]) -> Result<ExitCode> {
    // Determine which preset to use
    let preset_to_use = match preset_name {
        Some(name) => {
            eprintln!("using preset <{}>", name);
            name
        }
        None => {
            // Try to find "default" preset
            match loader::find_preset_scoped("default", scope) {
                Ok(_) => {
                    eprintln!("using preset <default>");
                    "default"
                }
                Err(_) => {
                    // No default preset found - run claude directly
                    eprintln!("no preset found, falling back to: claude");
                    return run_claude_directly(claude_args);
                }
            }
        }
    };

    let preset_path = loader::find_preset_scoped(preset_to_use, scope)
        .with_context(|| format!("Could not find preset: {}", preset_to_use))?;

    let preset = loader::load_preset(&preset_path)
        .with_context(|| format!("Could not load preset: {}", preset_to_use))?;

    let resolved = resolver::resolve_inheritance(&preset)
        .with_context(|| format!("Could not resolve preset: {}", preset_to_use))?;

    let mut cmd = Command::new("claude");

    for (key, value) in &resolved.env {
        cmd.env(key, value);
    }

    for arg in &resolved.args {
        cmd.arg(arg);
    }

    for arg in claude_args {
        cmd.arg(arg);
    }

    if let Some(prompt) = &resolved.prompt {
        cmd.arg(prompt);
    }

    let status = cmd.status()?;
    let code = status.code().unwrap_or(1);
    Ok(ExitCode::from(code.clamp(0, 255) as u8))
}

fn run_claude_directly(claude_args: &[String]) -> Result<ExitCode> {
    let mut cmd = Command::new("claude");
    cmd.args(claude_args);

    let status = cmd
        .status()
        .with_context(|| "Failed to execute claude command")?;

    let code = status.code().unwrap_or(1);
    Ok(ExitCode::from(code.clamp(0, 255) as u8))
}
