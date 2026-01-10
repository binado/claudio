use crate::preset;
use crate::scope::Scope;
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

/// Load an inline preset by name, honoring the same scope precedence as file presets.
///
/// For `Scope::Auto`:
/// 1) Check project settings (if present)
/// 2) If project settings has `ignore_user_presets: true`, do not consult user settings
/// 3) Check user settings
pub fn load_inline_preset_scoped(
    name: &str,
    scope: Scope,
) -> Result<Option<crate::preset::types::Preset>> {
    match scope {
        Scope::Project => load_inline_preset(name, Scope::Project),
        Scope::User => load_inline_preset(name, Scope::User),
        Scope::Auto => {
            // Project-first
            let project_settings = match get_settings_path(Scope::Project) {
                Ok(project_path) => {
                    if project_path.exists() {
                        load_settings(Scope::Project)?
                    } else {
                        None
                    }
                }
                Err(_) => None,
            };

            if let Some(project_settings) = project_settings {
                if let Some(preset) = project_settings.presets.iter().find(|p| p.name == name) {
                    return Ok(Some(preset.clone()));
                }

                if project_settings.ignore_user_presets.unwrap_or(false) {
                    return Ok(None);
                }
            }

            // Fall back to user
            load_inline_preset(name, Scope::User)
        }
    }
}

/// Load all inline presets visible to a scope, including their origin.
///
/// For `Scope::Auto`:
/// - Project presets come first and shadow user presets with the same name.
/// - If project settings has `ignore_user_presets: true`, user presets are not included.
pub fn load_inline_presets_scoped(
    scope: Scope,
) -> Result<Vec<(Scope, crate::preset::types::Preset)>> {
    let mut out: Vec<(Scope, crate::preset::types::Preset)> = Vec::new();

    match scope {
        Scope::Project => {
            if let Some(settings) = load_settings(Scope::Project)? {
                out.extend(settings.presets.into_iter().map(|p| (Scope::Project, p)));
            }
            Ok(out)
        }
        Scope::User => {
            if let Some(settings) = load_settings(Scope::User)? {
                out.extend(settings.presets.into_iter().map(|p| (Scope::User, p)));
            }
            Ok(out)
        }
        Scope::Auto => {
            let mut ignore_user = false;
            let mut seen_names: std::collections::HashSet<String> =
                std::collections::HashSet::new();

            let project_settings = match get_settings_path(Scope::Project) {
                Ok(project_path) => {
                    if project_path.exists() {
                        load_settings(Scope::Project)?
                    } else {
                        None
                    }
                }
                Err(_) => None,
            };

            if let Some(project_settings) = project_settings {
                ignore_user = project_settings.ignore_user_presets.unwrap_or(false);
                for p in project_settings.presets {
                    seen_names.insert(p.name.clone());
                    out.push((Scope::Project, p));
                }
            }

            if ignore_user {
                return Ok(out);
            }

            if let Some(user_settings) = load_settings(Scope::User)? {
                for p in user_settings.presets {
                    if seen_names.contains(&p.name) {
                        continue;
                    }
                    out.push((Scope::User, p));
                }
            }

            Ok(out)
        }
    }
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

/// Resolve the dry_run_show_env setting considering scope precedence
///
/// For `Scope::Auto`: project settings take precedence over user settings
pub fn resolve_dry_run_show_env(scope: Scope) -> Result<Option<bool>> {
    match scope {
        Scope::Project => Ok(load_settings(Scope::Project)?.and_then(|s| s.dry_run_show_env)),
        Scope::User => Ok(load_settings(Scope::User)?.and_then(|s| s.dry_run_show_env)),
        Scope::Auto => {
            if let Some(project_settings) = load_settings(Scope::Project)?
                && let Some(show_env) = project_settings.dry_run_show_env
            {
                return Ok(Some(show_env));
            }
            Ok(load_settings(Scope::User)?.and_then(|s| s.dry_run_show_env))
        }
    }
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
