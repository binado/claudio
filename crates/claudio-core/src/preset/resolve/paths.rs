use crate::paths;
use crate::preset::types::PresetSource;
use anyhow::Result;
use std::path::PathBuf;

/// Resolve a file path in a preset's env value relative to its source.
///
/// Path resolution rules:
/// - `~` is expanded to the OS home directory (not CLAUDIO_HOME_DIR)
/// - Absolute paths are used as-is
/// - Relative paths are resolved relative to:
///   - The preset file's directory (for file-based presets)
///   - The current working directory (for inline presets)
pub fn resolve_file_path_for_source(path: &str, source: &PresetSource) -> Result<PathBuf> {
    let expanded = paths::expand_tilde_os_home(path)?;

    if expanded.is_absolute() {
        return Ok(expanded);
    }

    match source {
        PresetSource::File(preset_path) => {
            let preset_dir = preset_path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Invalid preset path: no parent directory"))?;
            Ok(preset_dir.join(expanded))
        }
        PresetSource::Inline => Ok(std::env::current_dir()?.join(expanded)),
    }
}
