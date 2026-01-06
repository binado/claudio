use crate::cli::Scope;
use crate::preset::loader;
use anyhow::{Context, Result};
use std::env;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

pub fn edit(preset_name: &str, scope: Scope) -> Result<()> {
    let preset_path = match scope {
        Scope::Auto => {
            let matches = loader::find_all_presets_scoped(preset_name, Scope::Auto)?;
            match matches.len() {
                0 => create_preset_scoped(preset_name, Scope::Auto)?,
                1 => matches
                    .into_iter()
                    .next()
                    .expect("matches.len() == 1 implies one element"),
                _ => {
                    let locations: Vec<_> = matches
                        .iter()
                        .map(|p| format!("  - {}", p.display()))
                        .collect();
                    anyhow::bail!(
                        "Preset '{}' exists in multiple locations:\n{}\n\nRe-run with --scope project or --scope user to choose which one to edit.",
                        preset_name,
                        locations.join("\n")
                    );
                }
            }
        }
        Scope::Project | Scope::User => {
            let matches = loader::find_all_presets_scoped(preset_name, scope)?;
            match matches.len() {
                0 => create_preset_scoped(preset_name, scope)?,
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
        .with_context(|| format!("Failed to open editor '{}'", editor))?;

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
    } else {
        let detail = if let Some(code) = status.code() {
            format!("exit code {}", code)
        } else {
            #[cfg(unix)]
            {
                status
                    .signal()
                    .map(|sig| format!("signal {}", sig))
                    .unwrap_or_else(|| "unknown termination".to_string())
            }
            #[cfg(not(unix))]
            {
                "unknown termination".to_string()
            }
        };

        anyhow::bail!(
            "Editor '{}' failed ({})\n\nYou can change the editor by setting the VISUAL or EDITOR environment variable.",
            editor,
            detail
        );
    }

    Ok(())
}

fn create_preset_scoped(preset_name: &str, scope: Scope) -> Result<std::path::PathBuf> {
    let new_preset = loader::default_preset(preset_name);

    let target_dir = loader::get_preset_write_dir_scoped(scope)?;

    // Ensure directory exists before writing.
    std::fs::create_dir_all(&target_dir).with_context(|| {
        format!(
            "Failed to create preset directory: {}",
            target_dir.display()
        )
    })?;

    let new_path = loader::preset_path_for_name(&target_dir, preset_name);

    // Atomic write to avoid partial files on interruption.
    loader::write_preset_atomic(&new_path, &new_preset)
        .with_context(|| format!("Failed to write preset file: {}", new_path.display()))?;

    Ok(new_path)
}
