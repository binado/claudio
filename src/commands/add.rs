use crate::cli::Scope;
use crate::preset::loader;
use anyhow::{Context, Result};

use super::editor::open_in_editor;

pub fn add(preset_name: &str, scope: Scope) -> Result<()> {
    // 1. Check if preset already exists
    let existing = loader::find_all_presets_scoped(preset_name, scope)?;
    if !existing.is_empty() {
        let existing_paths = existing
            .iter()
            .map(|p| format!("  - {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");

        if matches!(scope, Scope::Auto) {
            anyhow::bail!(
                "Preset '{}' already exists (in user and/or project scope) at:\n{}\n\n\
                 If you want to create a preset with this name in a specific scope, \
                 rerun this command with the '--scope' flag (for example, '--scope user' or '--scope project').",
                preset_name,
                existing_paths
            );
        } else {
            anyhow::bail!(
                "Preset '{}' already exists at:\n{}",
                preset_name,
                existing_paths
            );
        }
    }

    // 2. Create the new preset file
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

    // 3. Open in editor
    open_in_editor(&new_path)
}
