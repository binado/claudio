use crate::cli::Scope;
use crate::preset::types::Preset;
use anyhow::{Context, Result};
use directories::BaseDirs;
use std::path::{Path, PathBuf};

pub fn discover_presets() -> Result<Vec<PathBuf>> {
    let mut presets = Vec::new();
    for dir in get_preset_dirs() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if entry
                    .path()
                    .extension()
                    .map(|e| e == "json")
                    .unwrap_or(false)
                {
                    presets.push(entry.path());
                }
            }
        }
    }
    presets.sort();
    presets.dedup();
    Ok(presets)
}

pub fn discover_presets_scoped(scope: Scope) -> Result<Vec<PathBuf>> {
    let mut presets = Vec::new();
    for dir in get_preset_dirs_scoped(scope)? {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if entry
                    .path()
                    .extension()
                    .map(|e| e == "json")
                    .unwrap_or(false)
                {
                    presets.push(entry.path());
                }
            }
        }
    }
    presets.sort();
    presets.dedup();
    Ok(presets)
}

pub fn load_preset(path: &Path) -> Result<Preset> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read preset file: {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse preset file: {}", path.display()))
}

pub fn default_preset(name: &str) -> Preset {
    Preset {
        name: name.to_string(),
        description: Some("Description of this preset".to_string()),
        extends: None,
        prompt: Some(String::new()),
        env: Some(std::collections::HashMap::new()),
        args: Some(Vec::new()),
        settings: None,
    }
}

pub fn preset_path_for_name(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{}.json", name))
}

pub fn write_preset_atomic(path: &Path, preset: &Preset) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "Could not determine parent directory for preset file: {}",
            path.display()
        )
    })?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Could not create preset directory: {}", parent.display()))?;

    let json = serde_json::to_string_pretty(preset)?;

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid preset filename: {}", path.display()))?;

    let tmp_path = parent.join(format!(".{}.tmp", file_name));
    std::fs::write(&tmp_path, json)
        .with_context(|| format!("Could not write preset file: {}", tmp_path.display()))?;

    std::fs::rename(&tmp_path, path)
        .with_context(|| format!("Could not replace preset file: {}", path.display()))?;

    Ok(())
}

pub fn find_preset(name: &str) -> Result<PathBuf> {
    for dir in get_preset_dirs() {
        let path = preset_path_for_name(&dir, name);
        if path.exists() {
            return Ok(path);
        }
    }
    anyhow::bail!("Preset '{}' not found", name)
}

pub fn find_preset_scoped(name: &str, scope: Scope) -> Result<PathBuf> {
    for dir in get_preset_dirs_scoped(scope)? {
        let path = preset_path_for_name(&dir, name);
        if path.exists() {
            return Ok(path);
        }
    }
    anyhow::bail!("Preset '{}' not found in scope {:?}", name, scope)
}

pub fn find_all_presets(name: &str) -> Vec<PathBuf> {
    let mut matches = Vec::new();
    for dir in get_preset_dirs() {
        let path = preset_path_for_name(&dir, name);
        if path.exists() {
            matches.push(path);
        }
    }
    matches
}

pub fn find_all_presets_scoped(name: &str, scope: Scope) -> Result<Vec<PathBuf>> {
    let mut matches = Vec::new();
    for dir in get_preset_dirs_scoped(scope)? {
        let path = preset_path_for_name(&dir, name);
        if path.exists() {
            matches.push(path);
        }
    }
    Ok(matches)
}

/// Returns the directory where new presets should be created/edited for a given scope.
///
/// - `Scope::Project`: `<project-root>/.claudio/presets`
/// - `Scope::User`: `~/.claudio/presets`
/// - `Scope::Auto`: prefers project-local `<project-root>/.claudio/presets` when a project root is detected, otherwise user
pub fn get_preset_write_dir_scoped(scope: Scope) -> Result<PathBuf> {
    match scope {
        Scope::Project => get_project_preset_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine project root")),

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

/// Returns the directory where new presets should be created/edited.
///
/// Preference order:
/// 1) Project-local `<project-root>/.claudio/presets` (if we can determine a project root)
/// 2) User-global `~/.claudio/presets`
pub fn get_preset_write_dir() -> Result<PathBuf> {
    get_preset_write_dir_scoped(Scope::Auto)
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
                    .ok_or_else(|| anyhow::anyhow!("Could not determine project root"))?,
            );
        }
        Scope::User => {
            dirs.push(get_user_preset_dir()?);
        }
    }

    Ok(dirs)
}

pub fn get_preset_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(project_dir) = get_project_preset_dir() {
        dirs.push(project_dir);
    } else if let Ok(cwd) = std::env::current_dir() {
        // Fallback to immediate CWD-based location if we can't detect a project root.
        dirs.push(cwd.join(".claudio").join("presets"));
    }

    if let Some(home) = BaseDirs::new() {
        dirs.push(home.home_dir().join(".claudio").join("presets"));
    }

    dirs
}

fn get_project_preset_dir() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let root = find_project_root(&cwd)?;
    Some(root.join(".claudio").join("presets"))
}

fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut dir = start;

    loop {
        // Treat either an existing `.claudio` directory or a `.git` directory as a project root marker.
        if dir.join(".claudio").is_dir() || dir.join(".git").is_dir() {
            return Some(dir.to_path_buf());
        }

        match dir.parent() {
            Some(parent) => dir = parent,
            None => return None,
        }
    }
}

fn get_user_preset_dir() -> Result<PathBuf> {
    let home = BaseDirs::new()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
        .home_dir()
        .to_owned();

    Ok(home.join(".claudio").join("presets"))
}
