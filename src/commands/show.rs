use crate::cli::Scope;
use crate::preset::loader;
use crate::preset::resolver;
use anyhow::{Context, Result};

pub fn show(preset_name: &str, scope: Scope, resolved: bool, json_only: bool) -> Result<()> {
    let preset_path = loader::find_preset_scoped(preset_name, scope)
        .with_context(|| format!("Could not find preset: {}", preset_name))?;

    let preset = loader::load_preset(&preset_path)
        .with_context(|| format!("Could not load preset: {}", preset_name))?;

    if json_only {
        let json = serde_json::to_string_pretty(&preset)?;
        println!("{}", json);
        return Ok(());
    }

    if resolved {
        let resolved_preset = resolver::resolve_inheritance(&preset)
            .with_context(|| format!("Could not resolve preset: {}", preset_name))?;

        println!("Configuration: {} (resolved)", preset_name);
        println!("Location: {}\n", preset_path.display());

        println!("Resolved Environment Variables:");
        for (key, value) in &resolved_preset.env {
            println!("  {}={}", key, value);
        }

        if let Some(prompt) = &resolved_preset.prompt {
            println!("\nPrompt:");
            println!("  {}", prompt);
        }

        println!("\nResolved CLI Arguments:");
        if resolved_preset.args.is_empty() {
            println!("  (none)");
        } else {
            for arg in &resolved_preset.args {
                println!("  {}", arg);
            }
        }

        println!("\nCommand that would be executed:");
        let args_str = resolved_preset.args.join(" ");
        println!("  claude {}", args_str);
    } else {
        println!("Configuration: {}", preset_name);
        println!("Location: {}\n", preset_path.display());

        let json = serde_json::to_string_pretty(&preset)?;
        println!("{}", json);
    }

    Ok(())
}
