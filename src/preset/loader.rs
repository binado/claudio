use crate::cli::Scope;
use crate::preset::types::Preset;
use crate::util::get_claudio_home;
use anyhow::{Context, Result};
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
    serde_json::from_str(&content).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse preset file: {}\n\nParse error: {}",
            path.display(),
            e
        )
    })
}

pub fn default_preset(name: &str) -> Preset {
    Preset {
        name: name.to_string(),
        description: Some("Description of this preset".to_string()),
        extends: None,
        prompt: Some(String::new()),
        env: Some(std::collections::HashMap::new()),
        args: Some(Vec::new()),
        settings: Some(serde_json::json!({})),
    }
}

pub fn write_preset_atomic(path: &Path, preset: &Preset) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to determine parent directory for preset file: {}",
            path.display()
        )
    })?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("Failed to create preset directory: {}", parent.display()))?;

    let json = serde_json::to_string_pretty(preset)?;

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid preset filename: {}", path.display()))?;

    let tmp_path = parent.join(format!(".{}.tmp", file_name));
    std::fs::write(&tmp_path, json)
        .with_context(|| format!("Failed to write preset file: {}", tmp_path.display()))?;

    std::fs::rename(&tmp_path, path)
        .with_context(|| format!("Failed to replace preset file: {}", path.display()))?;

    Ok(())
}

fn format_search_locations(dirs: &[PathBuf]) -> String {
    dirs.iter()
        .map(|p| format!("  - {}", p.display()))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn find_preset(name: &str) -> Result<PathBuf> {
    let dirs = get_preset_dirs();
    for dir in &dirs {
        let path = preset_path_for_name(dir, name);
        if path.exists() {
            return Ok(path);
        }
    }
    anyhow::bail!(
        "Preset '{}' not found.\n\nSearched in:\n{}\n\nRun `claudio init` to create a presets directory.",
        name,
        format_search_locations(&dirs)
    )
}

/// Try to find a preset - first checking inline presets in settings, then file-based presets
pub fn find_preset_or_inline(name: &str, scope: Scope) -> Result<Option<Preset>> {
    // First try inline preset from settings
    if let Some(inline_preset) = crate::settings::loader::load_inline_preset(name, scope)? {
        return Ok(Some(inline_preset));
    }

    // Then try file-based preset
    if let Ok(path) = find_preset_scoped(name, scope) {
        let preset = load_preset(&path)?;
        return Ok(Some(preset));
    }

    Ok(None)
}

pub fn find_preset_scoped(name: &str, scope: Scope) -> Result<PathBuf> {
    let dirs = get_preset_dirs_scoped(scope)?;
    for dir in &dirs {
        let path = preset_path_for_name(dir, name);
        if path.exists() {
            return Ok(path);
        }
    }
    anyhow::bail!(
        "Preset '{}' not found.\n\nSearched in:\n{}\n\nRun `claudio init` to create a presets directory.",
        name,
        format_search_locations(&dirs)
    )
}

pub fn find_all_presets(name: &str) -> Vec<PathBuf> {
    let mut matches = Vec::new();
    for dir in get_preset_dirs() {
        let paths = candidate_preset_paths_for_name(&dir, name);
        for path in paths {
            if path.exists() {
                matches.push(path);
            }
        }
    }
    matches
}

pub fn find_all_presets_scoped(name: &str, scope: Scope) -> Result<Vec<PathBuf>> {
    let mut matches = Vec::new();
    for dir in get_preset_dirs_scoped(scope)? {
        let paths = candidate_preset_paths_for_name(&dir, name);
        for path in paths {
            if path.exists() {
                matches.push(path);
            }
        }
    }
    Ok(matches)
}

pub(crate) fn is_legacy_filename_safe(name: &str) -> bool {
    // Legacy behavior used `<name>.json` directly.
    // Only allow that mapping when it cannot traverse directories.
    if name.is_empty() {
        return false;
    }
    if name.contains('\0') {
        return false;
    }
    if name.contains('/') || name.contains('\\') {
        return false;
    }
    // Avoid special path components.
    if name == "." || name == ".." {
        return false;
    }
    true
}

pub(crate) fn preset_filename_stem_for_name(name: &str) -> String {
    // Percent-encode UTF-8 bytes into a filesystem-safe filename stem.
    // We leave RFC3986 "unreserved" bytes as-is: ALPHA / DIGIT / "-" / "." / "_" / "~".
    // Everything else is escaped as `%XX` (uppercase hex).
    //
    // This is deterministic and avoids path traversal even when names contain `/`.
    let bytes = name.as_bytes();
    let mut out = String::with_capacity(bytes.len());

    for &b in bytes {
        let unreserved = matches!(
            b,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~'
        );

        if unreserved {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(
                char::from_digit((b >> 4) as u32, 16)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
            out.push(
                char::from_digit((b & 0x0F) as u32, 16)
                    .unwrap()
                    .to_ascii_uppercase(),
            );
        }
    }

    // Avoid empty stems and special path components.
    if out.is_empty() {
        return "%00".to_string();
    }
    if out == "." {
        return "%2E".to_string();
    }
    if out == ".." {
        return "%2E%2E".to_string();
    }

    out
}

pub(crate) fn preset_path_for_name_encoded(dir: &std::path::Path, name: &str) -> PathBuf {
    let stem = preset_filename_stem_for_name(name);
    dir.join(format!("{}.json", stem))
}

pub(crate) fn preset_path_for_name_legacy(dir: &std::path::Path, name: &str) -> PathBuf {
    dir.join(format!("{}.json", name))
}

pub(crate) fn candidate_preset_paths_for_name(dir: &std::path::Path, name: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();

    let encoded = preset_path_for_name_encoded(dir, name);
    out.push(encoded.clone());

    if is_legacy_filename_safe(name) {
        let legacy = preset_path_for_name_legacy(dir, name);
        if legacy != encoded {
            out.push(legacy);
        }
    }

    out
}

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
                    .ok_or_else(|| anyhow::anyhow!("Failed to determine project root"))?,
            );
        }
        Scope::User => {
            dirs.push(get_user_preset_dir()?);
        }
    }

    Ok(dirs)
}

/// Get the preset path for a given name under a directory.
///
/// If the name is "clean" (unreserved characters only and not `.`/`..`), this will
/// naturally produce `<dir>/<name>.json`.
///
/// For other names, it produces a percent-encoded filename.
pub fn preset_path_for_name(dir: &std::path::Path, name: &str) -> PathBuf {
    preset_path_for_name_encoded(dir, name)
}

pub fn get_preset_dirs() -> Vec<PathBuf> {
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
    let cwd = std::env::current_dir().ok()?;
    let root = find_project_root(&cwd)?;
    Some(root.join(".claudio").join("presets"))
}

/// Finds the project root by searching upward from a starting path.
///
/// This function walks up the directory tree starting from the `start` path until it locates
/// a project root marker (either a `.claudio` or `.git` directory). It stops and returns `None`
/// if it reaches the filesystem root without finding any markers.
///
/// # Arguments
/// * `start` - The starting directory to search from.
///
/// # Returns
/// * `Option<PathBuf>` - `Some(root)` when a project root is found, `None` otherwise.
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
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
    let home = get_claudio_home()?;
    Ok(home.join(".claudio").join("presets"))
}
