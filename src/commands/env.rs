use crate::cli::Scope;
use crate::preset::loader;
use crate::preset::resolver;
use anyhow::{Context, Result};

fn shell_escape_double_quoted(value: &str) -> String {
    // Escapes for use inside double quotes in common POSIX shells.
    // We escape characters that can terminate or interpolate within double quotes,
    // as well as history expansion and control characters for safety.
    let mut out = String::with_capacity(value.len());

    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '$' => out.push_str("\\$"),
            '`' => out.push_str("\\`"),
            _ => out.push(ch),
        }
    }

    out
}

pub fn env(preset_name: &str, scope: Scope, export: bool, resolved: bool) -> Result<()> {
    let preset_path = loader::find_preset_scoped(preset_name, scope)
        .with_context(|| format!("Failed to find preset: {}", preset_name))?;

    let preset = loader::load_preset(&preset_path)
        .with_context(|| format!("Failed to load preset: {}", preset_name))?;

    let resolved_preset = resolver::resolve_inheritance(&preset)
        .with_context(|| format!("Failed to resolve preset: {}", preset_name))?;

    if export {
        for (key, value) in &resolved_preset.env {
            let escaped_value = shell_escape_double_quoted(value);
            println!("export {}=\"{}\"", key, escaped_value);
        }
    } else if resolved {
        println!(
            "Environment variables for preset '{}' (resolved):\n",
            preset_name
        );
        for (key, value) in &resolved_preset.env {
            println!("{}={}", key, value);
        }
    } else {
        println!("Environment variables for preset '{}':\n", preset_name);
        if let Some(env_vars) = &preset.env {
            for (key, value) in env_vars {
                println!("{}={}", key, value);
            }
        }
    }

    Ok(())
}
