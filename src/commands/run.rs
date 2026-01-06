use crate::cli::Scope;
use crate::color::ColorConfig;
use crate::preset::loader;
use crate::preset::resolver;
use crate::settings::loader as settings_loader;
use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, ExitCode};

pub fn run(
    preset_name: Option<&str>,
    scope: Scope,
    claude_args: &[String],
    color_config: &ColorConfig,
) -> Result<ExitCode> {
    // Determine which preset to use
    let preset_to_use = match preset_name {
        Some(name) => {
            eprintln!(
                "using preset {}",
                color_config.highlight(&format!("<{}>", name))
            );
            name.to_string()
        }
        None => {
            // Try to get default preset from settings
            if let Some(default_name) = settings_loader::resolve_default_preset(scope)? {
                eprintln!(
                    "using preset {} (from settings)",
                    color_config.highlight(&format!("<{}>", default_name))
                );
                default_name
            } else if loader::find_preset_scoped("default", scope).is_ok() {
                // Fall back to "default" preset file
                eprintln!("using preset {}", color_config.highlight("<default>"));
                "default".to_string()
            } else {
                // No default preset found - run claude directly
                eprintln!("no preset found, falling back to: claude");
                return run_claude_directly(claude_args);
            }
        }
    };

    // Try to find the preset (inline or file-based)
    let preset = loader::find_preset_or_inline(&preset_to_use, scope)?
        .with_context(|| format!("Failed to find preset: {}", preset_to_use))?;

    let resolved = resolver::resolve_inheritance(&preset)
        .with_context(|| format!("Failed to resolve preset: {}", preset_to_use))?;

    let mut cmd = Command::new("claude");

    for (key, value) in &resolved.env {
        cmd.env(key, value);
    }

    for arg in &resolved.args {
        cmd.arg(arg);
    }

    // Add settings via temporary file if present (more secure than command-line args)
    let _temp_settings_file = if let Some(settings) = &resolved.settings {
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

    execute_claude_command(cmd)
}

fn run_claude_directly(claude_args: &[String]) -> Result<ExitCode> {
    let mut cmd = Command::new("claude");
    cmd.args(claude_args);
    execute_claude_command(cmd)
}

fn execute_claude_command(mut cmd: Command) -> Result<ExitCode> {
    let status = cmd.status().with_context(|| {
        "Failed to execute 'claude' command.\n\n\
         Make sure Claude Code is installed and available in your PATH.\n\
         See: https://github.com/anthropics/claude-code"
    })?;
    let code = status.code().unwrap_or(1);

    // temp_settings_file is dropped here, cleaning up the temporary file

    Ok(ExitCode::from(code.clamp(0, 255) as u8))
}
