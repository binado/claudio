use crate::cli::Scope;
use crate::preset::loader;
use crate::preset::store::PresetStore;
use crate::preset::types::PresetSource;
use anyhow::Result;

use super::editor::open_in_editor;

pub fn edit(preset_name: &str, scope: Scope) -> Result<()> {
    // Create a PresetStore for scope-aware lookups
    let store = PresetStore::new(scope)?;

    // In Auto scope, avoid silently choosing which file to edit if the same preset
    // exists in both project and user scope.
    if store.scope() == Scope::Auto {
        let mut file_paths = Vec::new();
        for dir in store.search_dirs() {
            for path in loader::candidate_preset_paths_for_name(dir, preset_name) {
                if path.exists() {
                    file_paths.push(path);
                }
            }
        }

        // Deduplicate paths in case encoded and legacy are the same.
        file_paths.sort();
        file_paths.dedup();

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
