use crate::cli::Scope;
use crate::preset;
use crate::settings::types::Settings;
use anyhow::{Context, Result};
use directories::BaseDirs;
use std::path::PathBuf;

/// Get the path to the settings file for a given scope
///
/// - Project: `<project-root>/.claudio/settings.json`
/// - User: `~/.claudio/settings.json`
/// - Auto: Prefer project if it exists, otherwise user
pub fn get_settings_path(scope: Scope) -> Result<PathBuf> {
    match scope {
        Scope::Project => get_project_settings_path()
            .ok_or_else(|| anyhow::anyhow!("Failed to determine project root")),
        Scope::User => Ok(get_user_settings_path()?),
        Scope::Auto => {
            // For Auto scope, we prefer project if it exists
            if let Some(project_path) = get_project_settings_path()
                && project_path.exists()
            {
                return Ok(project_path);
            }
            Ok(get_user_settings_path()?)
        }
    }
}

/// Load settings from the appropriate file (based on scope)
///
/// Returns `None` if the settings file doesn't exist.
pub fn load_settings(scope: Scope) -> Result<Option<Settings>> {
    let path = get_settings_path(scope)?;
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read settings file: {}", path.display()))?;

    let settings: Settings = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse settings file: {}", path.display()))?;

    settings.validate(scope)?;
    Ok(Some(settings))
}

/// Resolve the default preset name considering scope precedence
///
/// For `Scope::Auto`:
/// - If project settings exist and have a non-null default_preset, use it
/// - Otherwise, fall back to user settings default_preset
/// - If neither have a default, return None
pub fn resolve_default_preset(scope: Scope) -> Result<Option<String>> {
    match scope {
        Scope::Project => Ok(load_settings(Scope::Project)?.and_then(|s| s.default_preset)),
        Scope::User => Ok(load_settings(Scope::User)?.and_then(|s| s.default_preset)),
        Scope::Auto => {
            // Check project first, then fall back to user
            if let Some(project_settings) = load_settings(Scope::Project)?
                && let Some(default) = project_settings.default_preset
            {
                return Ok(Some(default));
            }

            // No project settings file exists, or project settings has no default_preset
            Ok(load_settings(Scope::User)?.and_then(|s| s.default_preset))
        }
    }
}

/// Try to load an inline preset from settings
///
/// Returns Some(preset) if found in settings, None if not in settings, Err if loading fails
pub fn load_inline_preset(
    name: &str,
    scope: Scope,
) -> Result<Option<crate::preset::types::Preset>> {
    if let Some(settings) = load_settings(scope)?
        && let Some(preset) = settings.presets.get(name)
    {
        return Ok(Some(preset.clone()));
    }
    Ok(None)
}

fn get_user_settings_path() -> Result<PathBuf> {
    let home = BaseDirs::new()
        .ok_or_else(|| anyhow::anyhow!("Failed to determine home directory"))?
        .home_dir()
        .to_owned();

    Ok(home.join(".claudio").join("settings.json"))
}

fn get_project_settings_path() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let root = preset::loader::find_project_root(&cwd)?;
    Some(root.join(".claudio").join("settings.json"))
}
