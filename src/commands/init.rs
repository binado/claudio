use crate::cli::Scope;
use crate::preset::loader;
use crate::settings::loader as settings_loader;
use crate::settings::types::Settings;
use anyhow::{Context, Result};

pub fn init(scope: Scope) -> Result<()> {
    // For init, we support initializing either the project directory or the
    // user directory. If the user passes `auto`, we treat it as `user` so that
    // `claudio init --scope auto` works even when no project root is detected.
    let scope = match scope {
        Scope::Auto => Scope::User,
        other => other,
    };

    let preset_dir = match scope {
        Scope::Project => loader::get_preset_write_dir_scoped(Scope::Project)?,
        Scope::User => loader::get_preset_write_dir_scoped(Scope::User)?,
        Scope::Auto => unreachable!("handled above"),
    };

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
