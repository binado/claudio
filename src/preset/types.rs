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

        if self.name.is_empty() {
            result.add_error("name field is required");
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
