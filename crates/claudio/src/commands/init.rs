use anyhow::{Context, Result};
use claudio_core::preset::loader;
use claudio_core::scope::Scope;
use claudio_core::settings::loader as settings_loader;
use claudio_core::settings::types::Settings;

pub fn init(scope: Scope) -> Result<()> {
    // For init, we support initializing either the project directory or the
    // user directory. If the user passes `auto`, we prefer project-local when
    // a project root is detected, otherwise fall back to user directory.
    let preset_dir = loader::get_preset_write_dir_scoped(scope)?;

    std::fs::create_dir_all(&preset_dir).with_context(|| {
        format!(
            "Failed to create preset directory: {}",
            preset_dir.display()
        )
    })?;

    // Create or update settings file
    let settings_path = settings_loader::get_settings_path(scope)?;
    if !settings_path.exists() {
        // Ensure the parent directory exists before writing
        if let Some(parent) = settings_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create settings directory: {}", parent.display())
            })?;
        }

        let default_settings = Settings::default();
        let json = serde_json::to_string_pretty(&default_settings)
            .context("Failed to serialize settings to JSON")?;

        std::fs::write(&settings_path, json).with_context(|| {
            format!("Failed to write settings file: {}", settings_path.display())
        })?;

        eprintln!("Created settings file: {}", settings_path.display());
    }

    Ok(())
}
