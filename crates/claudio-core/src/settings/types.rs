use crate::preset::loader;
use crate::preset::types::Preset;
use crate::scope::Scope;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    /// Name of the preset to use when no preset is specified
    #[serde(default)]
    pub default_preset: Option<String>,

    /// Inline preset definitions (can be used instead of separate files)
    #[serde(default)]
    pub presets: Vec<Preset>,

    /// Default highlight color (overrides CLI default)
    #[serde(default)]
    pub color: Option<String>,

    /// Disable colored output by default
    #[serde(default)]
    pub no_color: Option<bool>,

    /// If true, error when no preset is available instead of running bare claude
    #[serde(default)]
    pub require_preset: Option<bool>,

    /// If true, ignore user-level presets (only honored in project settings)
    #[serde(default)]
    pub ignore_user_presets: Option<bool>,

    /// If true, skip command confirmation prompts (commands will run without asking)
    /// Default is false (prompts enabled for security)
    #[serde(default)]
    pub skip_command_confirmation: Option<bool>,

    /// Whether to include environment variables in --dry-run output
    /// Default is true (show env vars)
    #[serde(default)]
    pub dry_run_show_env: Option<bool>,
}

impl Settings {
    /// Validate the settings file
    ///
    /// Checks:
    /// 1. No inline presets have the same name as file-based presets in the same scope
    /// 2. All inline presets are valid
    pub fn validate(&self, scope: Scope) -> Result<()> {
        // Check for name collisions between inline presets and file presets
        for preset in &self.presets {
            if let Ok(file_path) = loader::find_preset_scoped(&preset.name, scope) {
                anyhow::bail!(
                    "Inline preset '{}' conflicts with a file-based preset.\n\n\
                     Choose a different name or remove the file preset at:\n  {}",
                    preset.name,
                    file_path.display()
                );
            }
        }

        // Validate each inline preset
        for preset in &self.presets {
            let name = &preset.name;
            let validation = preset.validate(None);
            if !validation.errors.is_empty() {
                let message = if validation.errors.len() == 1 {
                    format!(
                        "Inline preset '{}' has validation error: {}",
                        name, validation.errors[0]
                    )
                } else {
                    format!(
                        "Inline preset '{}' has multiple validation errors:\n  - {}",
                        name,
                        validation.errors.join("\n  - ")
                    )
                };
                anyhow::bail!(message);
            }
        }

        Ok(())
    }
}
