use crate::preset::dirs::get_preset_dirs_scoped;
use crate::scope::Scope;
use anyhow::Result;
use std::path::PathBuf;

/// Discover all preset files (*.json) in the directories for a given scope.
///
/// Returns a sorted, deduplicated list of preset file paths.
pub fn discover_presets_scoped(scope: Scope) -> Result<Vec<PathBuf>> {
    let mut presets: Vec<PathBuf> = get_preset_dirs_scoped(scope)?
        .into_iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flatten()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();

    presets.sort();
    presets.dedup();
    Ok(presets)
}
