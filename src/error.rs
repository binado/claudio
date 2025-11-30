use std::error::Error as StdError;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum AppError {
    ConfigFileNotFound(PathBuf),
    ConfigParseError(PathBuf),
    ProviderNotFound(String),
    ApiKeyNotFound(String),
    ProcessLaunchFailed(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::ConfigFileNotFound(path) => {
                write!(f, "Config file '{}' not found", path.display())
            }
            AppError::ConfigParseError(msg) => {
                write!(f, "Failed to parse config file {}", msg.display())
            }
            AppError::ProviderNotFound(name) => {
                write!(f, "Provider '{}' not found in config", name)
            }
            AppError::ApiKeyNotFound(provider) => {
                write!(f, "API key not found for provider '{}'", provider)
            }
            AppError::ProcessLaunchFailed(msg) => write!(f, "Failed to launch process: {}", msg),
        }
    }
}

impl StdError for AppError {}

pub type Result<T> = std::result::Result<T, AppError>;
