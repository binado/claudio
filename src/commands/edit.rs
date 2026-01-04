use crate::preset::loader;
use anyhow::{Context, Result};
use directories::BaseDirs;
use std::env;
use std::process::Command;

pub fn edit(preset_name: &str) -> Result<()> {
    let preset_path = match loader::find_preset(preset_name) {
        Ok(path) => path,
        Err(_) => {
            println!(
                "Preset '{}' does not exist. Creating new preset...",
                preset_name
            );
            let new_preset = serde_json::json!({
                "name": preset_name,
                "description": "Description of this preset",
                "prompt": "",
                "env": {},
                "args": []
            });

            let preset_dirs = loader::get_preset_dirs();
            let target_dir = if let Some(dir) = preset_dirs.last() {
                dir.clone()
            } else {
                let home = BaseDirs::new()
                    .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
                    .home_dir()
                    .to_owned();
                home.join(".claudio/presets")
            };

            std::fs::create_dir_all(&target_dir).with_context(|| {
                format!(
                    "Could not create preset directory: {}",
                    target_dir.display()
                )
            })?;

            let new_path = target_dir.join(format!("{}.json", preset_name));
            std::fs::write(&new_path, serde_json::to_string_pretty(&new_preset)?)
                .with_context(|| format!("Could not write preset file: {}", new_path.display()))?;

            println!("Created new preset at: {}", new_path.display());
            new_path
        }
    };

    let editor = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| {
            if cfg!(target_os = "windows") {
                "notepad".to_string()
            } else {
                "vi".to_string()
            }
        });

    let status = Command::new(&editor)
        .arg(&preset_path)
        .status()
        .with_context(|| format!("Could not open editor '{}'", editor))?;

    if status.success() {
        println!("\nValidating preset...");
        let preset = loader::load_preset(&preset_path)
            .with_context(|| format!("Preset file is invalid: {}", preset_path.display()))?;

        let validation = preset.validate(Some(&preset_path));

        // Print warnings
        for warning in &validation.warnings {
            eprintln!("Warning: {}", warning);
        }

        // Print errors and fail if any
        if !validation.is_valid() {
            for error in &validation.errors {
                eprintln!("Error: {}", error);
            }
            anyhow::bail!("Preset validation failed");
        }

        println!("Preset validated successfully.");
    } else {
        println!("Editor exited with non-zero status. Skipping validation.");
    }

    Ok(())
}
