use crate::preset::dirs::get_preset_dirs_scoped;
use crate::scope::Scope;
use anyhow::Result;
use std::path::PathBuf;

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
