use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents a value for an environment variable or setting field.
/// Supports both simple string values (for backward compatibility) and
/// structured value sources (env, file, command, or direct value).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum EnvValue {
    /// Direct string value - can be a literal or contain ${VAR} substitutions
    /// This variant maintains backward compatibility with existing presets
    Direct(String),

    /// Structured value source with explicit source specification
    Structured {
        #[serde(flatten)]
        source: EnvValueSource,
    },
}

/// Specifies where to get the value from for an environment variable or setting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "from")]
pub enum EnvValueSource {
    /// Direct value assignment
    #[serde(rename = "value")]
    Value { value: String },

    /// Reference to an environment variable
    #[serde(rename = "env")]
    Env { var: String },

    /// Read value from a file (path relative to preset file location)
    #[serde(rename = "file")]
    File { path: String },

    /// Execute a command and use its stdout
    /// Requires confirmation by default.
    ///
    /// `claudio-core` does not perform terminal I/O. Callers must provide a
    /// confirmation handler (via `ResolverConfig` / `ConfirmCommand`) when
    /// confirmation is required; otherwise resolution will return an error.
    #[serde(rename = "command")]
    Command {
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        confirm: Option<bool>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub description: Option<String>,
    pub extends: Option<String>,
    pub prompt: Option<String>,
    pub env: Option<HashMap<String, EnvValue>>,
    pub args: Option<Vec<String>>,
    pub settings: Option<serde_json::Value>,
}

/// The origin of a preset - either inline in settings or from a file
#[derive(Debug, Clone)]
pub enum PresetSource {
    /// Inline preset defined in settings (no file on disk)
    Inline,
    /// File-based preset with its path
    File(PathBuf),
}

impl PresetSource {
    /// Returns the file path if this is a file-based preset
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            PresetSource::Inline => None,
            PresetSource::File(path) => Some(path),
        }
    }

    /// Returns true if this is an inline preset
    pub fn is_inline(&self) -> bool {
        matches!(self, PresetSource::Inline)
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedPreset {
    pub name: String,
    pub description: Option<String>,
    pub prompt: Option<String>,
    pub env: HashMap<String, String>,
    pub args: Vec<String>,
    pub settings: Option<serde_json::Value>,
    pub source: PresetSource,
}

#[derive(Debug, Clone, Default)]
pub struct ValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }
}

impl Preset {
    pub fn validate(&self, _path: Option<&std::path::Path>) -> ValidationResult {
        let mut result = ValidationResult::new();

        if self.name.trim().is_empty() {
            result.add_error("name field is required");
        }

        if let Some(extends) = self.extends.as_deref() {
            if extends.trim().is_empty() {
                result.add_error("extends field must not be empty when provided");
            } else if extends == self.name {
                result.add_error("extends field must not reference itself");
            }
        }

        if let Some(env) = &self.env {
            for (key, value) in env {
                if key.trim().is_empty() {
                    result.add_error("env contains an empty key");
                    continue;
                }

                if !is_valid_env_key(key) {
                    result.add_error(format!(
                        "env key '{}' is not a valid environment variable name",
                        key
                    ));
                }

                // Validate the value based on its type
                match value {
                    EnvValue::Direct(s) => {
                        if s.is_empty() {
                            result.add_warning(format!("env key '{}' has an empty value", key));
                        }
                    }
                    EnvValue::Structured { source } => match source {
                        EnvValueSource::Value { value: v } => {
                            if v.is_empty() {
                                result.add_warning(format!("env key '{}' has an empty value", key));
                            }
                        }
                        EnvValueSource::Env { var } => {
                            if !is_valid_env_key(var) {
                                result.add_error(format!(
                                    "env key '{}': variable name '{}' is invalid",
                                    key, var
                                ));
                            }
                        }
                        EnvValueSource::File { path } => {
                            if path.trim().is_empty() {
                                result.add_error(format!(
                                    "env key '{}': file path cannot be empty",
                                    key
                                ));
                            }
                        }
                        EnvValueSource::Command { command, .. } => {
                            if command.trim().is_empty() {
                                result.add_error(format!(
                                    "env key '{}': command cannot be empty",
                                    key
                                ));
                            } else {
                                result.add_warning(format!(
                                    "env key '{}': command execution poses security risks",
                                    key
                                ));
                            }
                        }
                    },
                }
            }
        }

        if let Some(prompt) = self.prompt.as_deref()
            && prompt.is_empty()
        {
            result.add_warning("prompt is empty");
        }

        if let Some(settings) = &self.settings {
            // Validate that settings is an object (dict-like)
            if !settings.is_object() {
                result.add_error("settings must be a JSON object");
            }
        }

        result
    }
}

fn is_valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();

    let Some(first) = chars.next() else {
        return false;
    };

    // Conservative POSIX-ish env var validation: [A-Z_][A-Z0-9_]*
    if !(first == '_' || first.is_ascii_uppercase()) {
        return false;
    }

    chars.all(|c| c == '_' || c.is_ascii_uppercase() || c.is_ascii_digit())
}

// NOTE: preset names are intentionally not validated for naming conventions.
// Filenames on disk are derived from names via encoding in the loader module,
// e.g., via `loader::preset_path_for_name` and related helpers.

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────────
    // is_valid_env_key tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_is_valid_env_key_valid_cases() {
        assert!(is_valid_env_key("API_KEY"));
        assert!(is_valid_env_key("_UNDERSCORE"));
        assert!(is_valid_env_key("A1_B2_C3"));
        assert!(is_valid_env_key("ANTHROPIC_BASE_URL"));
        assert!(is_valid_env_key("_"));
        assert!(is_valid_env_key("A"));
    }

    #[test]
    fn test_is_valid_env_key_invalid_cases() {
        assert!(!is_valid_env_key("lowercase"));
        assert!(!is_valid_env_key("123START"));
        assert!(!is_valid_env_key(""));
        assert!(!is_valid_env_key("CONTAINS-HYPHEN"));
        assert!(!is_valid_env_key("has spaces"));
        assert!(!is_valid_env_key("MixedCase"));
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // ValidationResult tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_validation_result_new() {
        let result = ValidationResult::new();
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validation_result_is_valid() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid());

        result.add_warning("a warning");
        assert!(result.is_valid(), "warnings should not affect validity");

        result.add_error("an error");
        assert!(!result.is_valid(), "errors should make result invalid");
    }

    #[test]
    fn test_validation_result_add_error() {
        let mut result = ValidationResult::new();
        result.add_error("first error");
        result.add_error("second error");
        assert_eq!(result.errors.len(), 2);
        assert_eq!(result.errors[0], "first error");
        assert_eq!(result.errors[1], "second error");
    }

    #[test]
    fn test_validation_result_add_warning() {
        let mut result = ValidationResult::new();
        result.add_warning("first warning");
        result.add_warning("second warning");
        assert_eq!(result.warnings.len(), 2);
        assert_eq!(result.warnings[0], "first warning");
        assert_eq!(result.warnings[1], "second warning");
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Preset::validate tests
    // ─────────────────────────────────────────────────────────────────────────────

    fn minimal_preset(name: &str) -> Preset {
        Preset {
            name: name.to_string(),
            description: None,
            extends: None,
            prompt: None,
            env: None,
            args: None,
            settings: None,
        }
    }

    #[test]
    fn test_validate_empty_name() {
        let preset = minimal_preset("");
        let result = preset.validate(None);
        assert!(!result.is_valid());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("name field is required"))
        );
    }

    #[test]
    fn test_validate_whitespace_name() {
        let preset = minimal_preset("   ");
        let result = preset.validate(None);
        assert!(!result.is_valid());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("name field is required"))
        );
    }

    #[test]
    fn test_validate_self_referencing_extends() {
        let mut preset = minimal_preset("my-preset");
        preset.extends = Some("my-preset".to_string());
        let result = preset.validate(None);
        assert!(!result.is_valid());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("must not reference itself"))
        );
    }

    #[test]
    fn test_validate_empty_extends() {
        let mut preset = minimal_preset("my-preset");
        preset.extends = Some("".to_string());
        let result = preset.validate(None);
        assert!(!result.is_valid());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("extends field must not be empty"))
        );
    }

    #[test]
    fn test_validate_invalid_env_key() {
        let mut preset = minimal_preset("my-preset");
        let mut env = HashMap::new();
        env.insert(
            "lowercase_key".to_string(),
            EnvValue::Direct("value".to_string()),
        );
        preset.env = Some(env);
        let result = preset.validate(None);
        assert!(!result.is_valid());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("not a valid environment variable name"))
        );
    }

    #[test]
    fn test_validate_empty_env_value_warning() {
        let mut preset = minimal_preset("my-preset");
        let mut env = HashMap::new();
        env.insert("API_KEY".to_string(), EnvValue::Direct("".to_string()));
        preset.env = Some(env);
        let result = preset.validate(None);
        assert!(
            result.is_valid(),
            "empty env value should only be a warning"
        );
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("has an empty value"))
        );
    }

    #[test]
    fn test_validate_empty_prompt_warning() {
        let mut preset = minimal_preset("my-preset");
        preset.prompt = Some("".to_string());
        let result = preset.validate(None);
        assert!(result.is_valid(), "empty prompt should only be a warning");
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("prompt is empty"))
        );
    }

    #[test]
    fn test_validate_settings_not_object() {
        let mut preset = minimal_preset("my-preset");
        preset.settings = Some(serde_json::json!("not an object"));
        let result = preset.validate(None);
        assert!(!result.is_valid());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("settings must be a JSON object"))
        );
    }

    #[test]
    fn test_validate_settings_object_valid() {
        let mut preset = minimal_preset("my-preset");
        preset.settings = Some(serde_json::json!({"key": "value"}));
        let result = preset.validate(None);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_valid_preset() {
        let mut preset = minimal_preset("my-preset");
        preset.description = Some("A valid preset".to_string());
        preset.extends = Some("base".to_string());
        preset.prompt = Some("Do something".to_string());
        let mut env = HashMap::new();
        env.insert(
            "API_KEY".to_string(),
            EnvValue::Direct("${MY_API_KEY}".to_string()),
        );
        preset.env = Some(env);
        preset.args = Some(vec!["--verbose".to_string()]);
        preset.settings = Some(serde_json::json!({"model": "sonnet"}));

        let result = preset.validate(None);
        assert!(result.is_valid());
        assert!(result.warnings.is_empty());
    }
}
