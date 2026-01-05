use crate::cli::Scope;
use crate::preset::loader;
use crate::preset::resolver;
use anyhow::{Context, Result};
use std::io::Write;
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

    // Add settings via temporary file if present (more secure than command-line args)
    let temp_settings_file = if let Some(settings) = &resolved.settings {
        let settings_json = serde_json::to_string_pretty(settings)
            .context("Failed to serialize settings to JSON")?;

        // Create a temporary file in the system temp directory
        let mut temp_file =
            tempfile::NamedTempFile::new().context("Failed to create temporary settings file")?;

        // Write settings directly through the file handle (avoids race condition)
        temp_file
            .write_all(settings_json.as_bytes())
            .context("Failed to write settings to temporary file")?;

        let temp_path = temp_file.path().to_path_buf();
        cmd.arg("--settings");
        cmd.arg(&temp_path);

        Some(temp_file)
    } else {
        None
    };

    for arg in claude_args {
        cmd.arg(arg);
    }

    if let Some(prompt) = &resolved.prompt {
        cmd.arg(prompt);
    }

    let status = cmd.status()?;
    let code = status.code().unwrap_or(1);

    // Note: temp_settings_file is dropped here, which removes the temporary file
    // (tempfile::NamedTempFile cleans itself up on drop)
    drop(temp_settings_file);

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
