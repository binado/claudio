use crate::preset::types::Preset;
use anyhow::{Context, Result};
use std::path::Path;

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
        prompt: None,
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
