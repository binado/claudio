use crate::cli::Scope;
use crate::preset::loader;
use crate::preset::types::Preset;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    /// Name of the preset to use when no preset is specified
    #[serde(default)]
    pub default_preset: Option<String>,

    /// Inline preset definitions (can be used instead of separate files)
    #[serde(default)]
    pub presets: HashMap<String, Preset>,
}

impl Settings {
    /// Validate the settings file
    ///
    /// Checks:
    /// 1. No inline presets have the same name as file-based presets in the same scope
    /// 2. All inline presets are valid
    pub fn validate(&self, scope: Scope) -> Result<()> {
        // Check for name collisions between inline presets and file presets
        for preset_name in self.presets.keys() {
            if loader::find_preset_scoped(preset_name, scope).is_ok() {
                let file_path = loader::find_preset_scoped(preset_name, scope)?;
                anyhow::bail!(
                    "Inline preset '{}' conflicts with a file-based preset.\n\n\
                     Choose a different name or remove the file preset at:\n  {}",
                    preset_name,
                    file_path.display()
                );
            }
        }

        // Validate each inline preset
        for (name, preset) in &self.presets {
            let validation = preset.validate(None);
            if !validation.is_valid() {
                for error in validation.errors {
                    anyhow::bail!("Inline preset '{}' has validation error: {}", name, error);
                }
            }
        }

        Ok(())
    }
}
