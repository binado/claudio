use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json;

use crate::error::{AppError, Result};

#[derive(Serialize, Deserialize)]
pub struct Provider {
    pub api_endpoint: String,
    pub opus_model: String,
    pub sonnet_model: String,
    pub haiku_model: String,
}

#[derive(Serialize, Deserialize)]
pub struct ProviderConfig {
    pub providers: HashMap<String, Provider>,
}

impl Provider {
    pub fn get_env_vars(&self, api_key: String) -> Vec<(String, String)> {
        vec![
            ("ANTHROPIC_BASE_URL".to_string(), self.api_endpoint.clone()),
            (
                "ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(),
                self.opus_model.clone(),
            ),
            (
                "ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(),
                self.sonnet_model.clone(),
            ),
            (
                "ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(),
                self.haiku_model.clone(),
            ),
            ("ANTHROPIC_AUTH_TOKEN".to_string(), api_key),
        ]
    }
}

pub fn get_providers(filename: &Path) -> Result<HashMap<String, Provider>> {
    let json = fs::read_to_string(&filename).map_err(|_| AppError::ConfigFileNotFound(filename.to_path_buf()))?;
    let provider_config: ProviderConfig =
        serde_json::from_str(&json).map_err(|_| AppError::ConfigParseError(filename.to_path_buf()))?;
    Ok(provider_config.providers)
}
