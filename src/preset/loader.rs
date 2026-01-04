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

pub fn load_preset(path: &Path) -> Result<Preset> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read preset file: {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse preset file: {}", path.display()))
}

pub fn find_preset(name: &str) -> Result<PathBuf> {
    for dir in get_preset_dirs() {
        let path = dir.join(format!("{}.json", name));
        if path.exists() {
            return Ok(path);
        }
    }
    anyhow::bail!("Preset '{}' not found", name)
}

pub fn find_all_presets(name: &str) -> Vec<PathBuf> {
    let mut matches = Vec::new();
    for dir in get_preset_dirs() {
        let path = dir.join(format!("{}.json", name));
        if path.exists() {
            matches.push(path);
        }
    }
    matches
}

pub fn get_preset_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join(".claudio").join("presets"));
    }
    if let Some(home) = BaseDirs::new() {
        dirs.push(home.home_dir().join(".claudio").join("presets"));
    }
    dirs
}
