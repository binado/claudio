use crate::cli::Scope;
use crate::preset::store::PresetStore;
use crate::preset::types::{PresetSource, validate_preset_name};
use anyhow::Result;

use super::editor::open_in_editor;

pub fn edit(preset_name: &str, scope: Scope) -> Result<()> {
    // Validate preset name before any file operations
    let validation = validate_preset_name(preset_name);
    if !validation.is_safe {
        anyhow::bail!(
            "Invalid preset name '{}': contains path separators or traversal patterns.\n\n\
             Preset names must not contain: /, \\, .., or NUL characters.",
            preset_name
        );
    }

    // Create a PresetStore for scope-aware lookups
    let store = PresetStore::new(scope)?;

    // In Auto scope, avoid silently choosing which file to edit if the same preset
    // exists in both project and user scope.
    if store.scope() == Scope::Auto {
        let mut file_paths = Vec::new();
        for dir in store.search_dirs() {
            let path = dir.join(format!("{}.json", preset_name));
            if path.exists() {
                file_paths.push(path);
            }
        }

        match file_paths.len() {
            0 => {
                // No file preset; check if it exists inline.
                if let Some(loc) = store.find(preset_name)?
                    && matches!(loc.source, PresetSource::Inline)
                {
                    anyhow::bail!(
                        "Preset '{}' is an inline preset defined in settings.\n\n\
                         Inline presets cannot be edited with this command.\n\
                         Edit your settings file to modify inline presets.",
                        preset_name
                    );
                }

                anyhow::bail!(
                    "Preset '{}' not found. Use 'claudio preset add {}' to create it.",
                    preset_name,
                    preset_name
                );
            }
            1 => {
                return open_in_editor(&file_paths[0]);
            }
            _ => {
                let locations = file_paths
                    .iter()
                    .map(|p| format!("  - {}", p.display()))
                    .collect::<Vec<_>>()
                    .join("\n");
                anyhow::bail!(
                    "Preset '{}' exists in multiple locations:\n{}\n\n\
                     Re-run with --scope project or --scope user to choose which one to edit.",
                    preset_name,
                    locations
                );
            }
        }
    }

    // Find the preset
    let located = store.find(preset_name)?;

    let preset_path = match located {
        None => {
            anyhow::bail!(
                "Preset '{}' not found. Use 'claudio preset add {}' to create it.",
                preset_name,
                preset_name
            );
        }
        Some(loc) => match loc.source {
            PresetSource::Inline => {
                anyhow::bail!(
                    "Preset '{}' is an inline preset defined in settings.\n\n\
                         Inline presets cannot be edited with this command.\n\
                         Edit your settings file to modify inline presets.",
                    preset_name
                );
            }
            PresetSource::File(path) => path,
        },
    };

    // Open editor
    open_in_editor(&preset_path)
}
