use crate::paths;
use crate::preset::types::PresetSource;
use anyhow::Result;
use std::path::PathBuf;

/// Resolve a file path relative to the preset source location.
pub(crate) fn resolve_file_path(path: &str, source: &PresetSource) -> Result<PathBuf> {
    let path = paths::expand_tilde_os_home(path)?;

    if path.is_absolute() {
        return Ok(path);
    }

    match source {
        PresetSource::File(preset_path) => {
            let preset_dir = preset_path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Invalid preset path: no parent directory"))?;
            Ok(preset_dir.join(path))
        }
        PresetSource::Inline => Ok(std::env::current_dir()?.join(path)),
    }
}

/// Resolve a file path in a preset's env value relative to its source.
///
/// This mirrors the behavior used by env resolution:
/// - `~` expansion (OS home only)
/// - absolute paths kept as-is
/// - relative paths resolved relative to the preset file directory (file presets)
///   or current directory (inline presets)
pub fn resolve_file_path_for_source(path: &str, source: &PresetSource) -> Result<PathBuf> {
    resolve_file_path(path, source)
}
