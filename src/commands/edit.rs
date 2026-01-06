use crate::cli::Scope;
use crate::preset::loader;
use anyhow::Result;

use super::editor::open_in_editor;

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
