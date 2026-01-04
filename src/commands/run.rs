use crate::preset::loader;
use crate::preset::resolver;
use anyhow::{Context, Result};
use std::process::Command;

pub fn run(preset_name: &str, claude_args: &[String]) -> Result<i32> {
    let preset_path = loader::find_preset(preset_name)
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
    Ok(status.code().unwrap_or(1))
}
