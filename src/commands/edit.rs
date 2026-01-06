use crate::cli::Scope;
use crate::preset::loader;
use anyhow::{Context, Result};
use std::env;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

pub fn edit(preset_name: &str, scope: Scope) -> Result<()> {
    // 1. Find the preset (error if not found)
    let preset_path = match scope {
        Scope::Auto => {
            let matches = loader::find_all_presets_scoped(preset_name, Scope::Auto)?;
            match matches.len() {
                0 => anyhow::bail!(
                    "Preset '{}' not found. Use 'claudio preset add {}' to create it.",
                    preset_name,
                    preset_name
                ),
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
            match loader::find_all_presets_scoped(preset_name, scope)?.len() {
                0 => anyhow::bail!(
                    "Preset '{}' not found in {} scope. Use 'claudio preset add {}' to create it.",
                    preset_name,
                    match scope {
                        Scope::Project => "project",
                        Scope::User => "user",
                        _ => unreachable!("Scope::Auto handled in outer match"),
                    },
                    preset_name
                ),
                _ => loader::find_preset_scoped(preset_name, scope)?,
            }
        }
    };

    // 2. Open editor
    open_in_editor(&preset_path)
}

fn open_in_editor(preset_path: &std::path::Path) -> Result<()> {
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
        .arg(preset_path)
        .status()
        .with_context(|| format!("Failed to open editor '{}'", editor))?;

    if status.success() {
        let preset = loader::load_preset(preset_path)
            .with_context(|| format!("Preset file is invalid: {}", preset_path.display()))?;

        let validation = preset.validate(Some(preset_path));

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
