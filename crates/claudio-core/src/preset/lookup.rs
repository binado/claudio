use crate::preset::dirs::get_preset_dirs;
use crate::preset::dirs::get_preset_dirs_scoped;
use crate::preset::naming::candidate_preset_paths_for_name;
use crate::preset::naming::preset_path_for_name;
use crate::scope::Scope;
use anyhow::Result;
use std::path::PathBuf;

fn format_search_locations(dirs: &[PathBuf]) -> String {
    dirs.iter()
        .map(|p| format!("  - {}", p.display()))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn find_preset(name: &str) -> Result<PathBuf> {
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
