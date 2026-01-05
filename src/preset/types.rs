use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub description: Option<String>,
    pub extends: Option<String>,
    pub prompt: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct ResolvedPreset {
    pub name: String,
    pub description: Option<String>,
    pub prompt: Option<String>,
    pub env: HashMap<String, String>,
    pub args: Vec<String>,
    pub source_path: PathBuf,
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
    pub fn validate(&self, path: Option<&std::path::Path>) -> ValidationResult {
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

                if value.is_empty() {
                    result.add_warning(format!("env key '{}' has an empty value", key));
                }
            }
        }

        if let Some(prompt) = self.prompt.as_deref() {
            if prompt.is_empty() {
                result.add_warning("prompt is empty");
            }
        }

        if let Some(path) = path
            && let Some(file_stem) = path.file_stem().and_then(|s| s.to_str())
            && self.name != file_stem
        {
            result.add_warning(format!(
                "Preset name '{}' does not match filename '{}'",
                self.name, file_stem
            ));
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
