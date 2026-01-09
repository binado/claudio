use anyhow::{Context, Result};
use claudio_core::preset::loader;
use claudio_core::preset::store::PresetStore;
use claudio_core::scope::Scope;

use super::editor::open_in_editor;

pub fn add(preset_name: &str, scope: Scope) -> Result<()> {
    // Create a PresetStore for scope-aware lookups
    let store = PresetStore::new(scope)?;

    // 1. Check if preset already exists
    if store.find(preset_name)?.is_some() {
        if matches!(scope, Scope::Auto) {
            anyhow::bail!(
                "Preset '{}' already exists (in user and/or project scope).\n\n\
                 If you want to create a preset with this name in a specific scope, \
                 rerun this command with the '--scope' flag (for example, '--scope user' or '--scope project').",
                preset_name
            );
        } else {
            anyhow::bail!("Preset '{}' already exists.", preset_name);
        }
    }

    // 2. Create the new preset file
    let new_preset = loader::default_preset(preset_name);
    let target_dir = store.write_dir()?;

    // Ensure directory exists before writing.
    std::fs::create_dir_all(&target_dir).with_context(|| {
        format!(
            "Failed to create preset directory: {}",
            target_dir.display()
        )
    })?;

    let new_path = store.path_for_name(preset_name)?;

    // Atomic write to avoid partial files on interruption.
    loader::write_preset_atomic(&new_path, &new_preset)
        .with_context(|| format!("Failed to write preset file: {}", new_path.display()))?;

    // 3. Open in editor
    open_in_editor(&new_path)
}
