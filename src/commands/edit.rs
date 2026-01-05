use crate::cli::Scope;
use crate::preset::loader;
use crate::preset::types::Preset;
use anyhow::{Context, Result};
use std::env;
use std::process::Command;

pub fn edit(preset_name: &str, scope: Scope) -> Result<()> {
    let preset_path = match scope {
        Scope::Auto => {
            let matches = loader::find_all_presets_scoped(preset_name, Scope::Auto)?;
            match matches.len() {
                0 => {
                    let new_preset = Preset {
                        name: preset_name.to_string(),
                        description: Some("Description of this preset".to_string()),
                        extends: None,
                        prompt: Some(String::new()),
                        env: Some(std::collections::HashMap::new()),
                        args: Some(Vec::new()),
                    };

                    let target_dir = loader::get_preset_write_dir_scoped(Scope::Auto)?;

                    std::fs::create_dir_all(&target_dir).with_context(|| {
                        format!(
                            "Could not create preset directory: {}",
                            target_dir.display()
                        )
                    })?;

                    let new_path = target_dir.join(format!("{}.json", preset_name));
                    std::fs::write(&new_path, serde_json::to_string_pretty(&new_preset)?)
                        .with_context(|| {
                            format!("Could not write preset file: {}", new_path.display())
                        })?;

                    new_path
                }
                1 => matches
                    .into_iter()
                    .next()
                    .expect("matches.len() == 1 implies one element"),
                _ => {
                    anyhow::bail!(
                        "Preset '{}' exists in multiple scopes. Re-run with --scope project or --scope user to choose which one to edit.",
                        preset_name
                    );
                }
            }
        }
        Scope::Project | Scope::User => {
            let matches = loader::find_all_presets_scoped(preset_name, scope)?;
            match matches.len() {
                0 => {
                    let new_preset = Preset {
                        name: preset_name.to_string(),
                        description: Some("Description of this preset".to_string()),
                        extends: None,
                        prompt: Some(String::new()),
                        env: Some(std::collections::HashMap::new()),
                        args: Some(Vec::new()),
                    };

                    let target_dir = loader::get_preset_write_dir_scoped(scope)?;

                    std::fs::create_dir_all(&target_dir).with_context(|| {
                        format!(
                            "Could not create preset directory: {}",
                            target_dir.display()
                        )
                    })?;

                    let new_path = target_dir.join(format!("{}.json", preset_name));
                    std::fs::write(&new_path, serde_json::to_string_pretty(&new_preset)?)
                        .with_context(|| {
                            format!("Could not write preset file: {}", new_path.display())
                        })?;

                    new_path
                }
                _ => loader::find_preset_scoped(preset_name, scope)?,
            }
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
        let preset = loader::load_preset(&preset_path)
            .with_context(|| format!("Preset file is invalid: {}", preset_path.display()))?;

        let validation = preset.validate(Some(&preset_path));

        if !validation.is_valid() {
            for warning in &validation.warnings {
                eprintln!("Warning: {}", warning);
            }
            for error in &validation.errors {
                eprintln!("Error: {}", error);
            }
            anyhow::bail!("Preset validation failed");
        }
    }

    Ok(())
}
