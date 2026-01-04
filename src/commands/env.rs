use crate::preset::loader;
use crate::preset::resolver;
use anyhow::{Context, Result};

pub fn env(preset_name: &str, export: bool, resolved: bool) -> Result<()> {
    let preset_path = loader::find_preset(preset_name)
        .with_context(|| format!("Could not find preset: {}", preset_name))?;

    let preset = loader::load_preset(&preset_path)
        .with_context(|| format!("Could not load preset: {}", preset_name))?;

    let resolved_preset = resolver::resolve_inheritance(&preset)
        .with_context(|| format!("Could not resolve preset: {}", preset_name))?;

    if export {
        for (key, value) in &resolved_preset.env {
            println!("export {}=\"{}\"", key, value);
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
