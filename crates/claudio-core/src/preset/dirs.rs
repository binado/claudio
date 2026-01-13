use crate::paths::{find_project_root, get_claudio_home};
use crate::scope::Scope;
use anyhow::Result;
use std::path::PathBuf;

/// Returns the directory where new presets should be created/edited for a given scope.
///
/// - `Scope::Project`: `<project-root>/.claudio/presets`
/// - `Scope::User`: `~/.claudio/presets`
/// - `Scope::Auto`: prefers project-local `<project-root>/.claudio/presets` when a project root is detected, otherwise user
pub fn get_preset_write_dir_scoped(scope: Scope) -> Result<PathBuf> {
    match scope {
        Scope::Project => get_project_preset_dir()
            .ok_or_else(|| anyhow::anyhow!("Failed to determine project root")),

        Scope::User => Ok(get_user_preset_dir()?),

        Scope::Auto => {
            if let Some(project) = get_project_preset_dir() {
                Ok(project)
            } else {
                Ok(get_user_preset_dir()?)
            }
        }
    }
}

/// Returns preset directories for a given scope.
///
/// - `Scope::Auto`: `[project, user]` (project first; shadows user)
/// - `Scope::Project`: `[project]`
/// - `Scope::User`: `[user]`
pub fn get_preset_dirs_scoped(scope: Scope) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();

    match scope {
        Scope::Auto => {
            if let Some(project) = get_project_preset_dir() {
                dirs.push(project);
            }
            dirs.push(get_user_preset_dir()?);
        }
        Scope::Project => {
            dirs.push(
                get_project_preset_dir()
                    .ok_or_else(|| anyhow::anyhow!("Failed to determine project root"))?,
            );
        }
        Scope::User => {
            dirs.push(get_user_preset_dir()?);
        }
    }

    Ok(dirs)
}

pub(crate) fn get_preset_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(project_dir) = get_project_preset_dir() {
        dirs.push(project_dir);
    } else if let Ok(cwd) = std::env::current_dir() {
        // Fallback to immediate CWD-based location if we can't detect a project root.
        dirs.push(cwd.join(".claudio").join("presets"));
    }

    if let Ok(user_dir) = get_user_preset_dir() {
        dirs.push(user_dir);
    }

    dirs
}

fn get_project_preset_dir() -> Option<PathBuf> {
    let root = find_project_root(None)?;
    Some(root.join(".claudio").join("presets"))
}

fn get_user_preset_dir() -> Result<PathBuf> {
    let home = get_claudio_home()?;
    Ok(home.join(".claudio").join("presets"))
}
