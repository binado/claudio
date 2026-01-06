use crate::cli::Scope;
use crate::preset;
use crate::settings::types::Settings;
use crate::util::get_claudio_home;
use anyhow::{Context, Result};
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
        && let Some(preset) = settings.presets.iter().find(|p| p.name == name)
    {
        return Ok(Some(preset.clone()));
    }
    Ok(None)
}

/// Resolve the color setting considering scope precedence
///
/// For `Scope::Auto`: project settings take precedence over user settings
pub fn resolve_color(scope: Scope) -> Result<Option<String>> {
    match scope {
        Scope::Project => Ok(load_settings(Scope::Project)?.and_then(|s| s.color)),
        Scope::User => Ok(load_settings(Scope::User)?.and_then(|s| s.color)),
        Scope::Auto => {
            if let Some(project_settings) = load_settings(Scope::Project)?
                && let Some(color) = project_settings.color
            {
                return Ok(Some(color));
            }
            Ok(load_settings(Scope::User)?.and_then(|s| s.color))
        }
    }
}

/// Resolve the no_color setting considering scope precedence
///
/// For `Scope::Auto`: project settings take precedence over user settings
pub fn resolve_no_color(scope: Scope) -> Result<Option<bool>> {
    match scope {
        Scope::Project => Ok(load_settings(Scope::Project)?.and_then(|s| s.no_color)),
        Scope::User => Ok(load_settings(Scope::User)?.and_then(|s| s.no_color)),
        Scope::Auto => {
            if let Some(project_settings) = load_settings(Scope::Project)?
                && let Some(no_color) = project_settings.no_color
            {
                return Ok(Some(no_color));
            }
            Ok(load_settings(Scope::User)?.and_then(|s| s.no_color))
        }
    }
}

/// Resolve the require_preset setting considering scope precedence
///
/// For `Scope::Auto`: project settings take precedence over user settings
pub fn resolve_require_preset(scope: Scope) -> Result<Option<bool>> {
    match scope {
        Scope::Project => Ok(load_settings(Scope::Project)?.and_then(|s| s.require_preset)),
        Scope::User => Ok(load_settings(Scope::User)?.and_then(|s| s.require_preset)),
        Scope::Auto => {
            if let Some(project_settings) = load_settings(Scope::Project)?
                && let Some(require_preset) = project_settings.require_preset
            {
                return Ok(Some(require_preset));
            }
            Ok(load_settings(Scope::User)?.and_then(|s| s.require_preset))
        }
    }
}

/// Check if user presets should be ignored (only honored from project settings)
///
/// Returns true only if project settings exist and have `ignore_user_presets: true`
pub fn should_ignore_user_presets() -> Result<bool> {
    if let Some(project_settings) = load_settings(Scope::Project)? {
        return Ok(project_settings.ignore_user_presets.unwrap_or(false));
    }
    Ok(false)
}

fn get_user_settings_path() -> Result<PathBuf> {
    let home = get_claudio_home()?;
    Ok(home.join(".claudio").join("settings.json"))
}

fn get_project_settings_path() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let root = preset::loader::find_project_root(&cwd)?;
    Some(root.join(".claudio").join("settings.json"))
}
