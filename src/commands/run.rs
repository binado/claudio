use crate::cli::Scope;
use crate::preset::loader;
use crate::preset::resolver;
use anyhow::{Context, Result};
use std::process::{Command, ExitCode};

pub fn run(preset_name: &str, scope: Scope, claude_args: &[String]) -> Result<ExitCode> {
    let preset_path = loader::find_preset_scoped(preset_name, scope)
        .with_context(|| format!("Could not find preset: {}", preset_name))?;

    let preset = loader::load_preset(&preset_path)
        .with_context(|| format!("Could not load preset: {}", preset_name))?;

    let resolved = resolver::resolve_inheritance(&preset)
        .with_context(|| format!("Could not resolve preset: {}", preset_name))?;

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
