mod cli;
mod error;
mod provider;

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::process;

use crate::cli::get_cli;
use crate::error::{AppError, Result};

const CLAUDE_BINARY: &str = "claude";

fn get_config_file() -> PathBuf {
    let home_dir = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home_dir)
        .join(".claude")
        .join("providers.json")
}

fn launch_claude(vars: HashMap<String, String>, args: &[String]) -> Result<()> {
    let status = process::Command::new(CLAUDE_BINARY)
        .envs(&vars)
        .args(args)
        .status()
        .map_err(|e| AppError::ProcessLaunchFailed(e.to_string()))?;

    process::exit(status.code().unwrap_or(1));
}

fn main() -> Result<()> {
    let cli = get_cli();

    let provider = cli.provider.as_deref().unwrap_or("");
    if provider.is_empty() {
        let _ = launch_claude(HashMap::new(), &cli.args);
        return Ok(());
    }

    let config_file = cli.config_file.clone().unwrap_or_else(get_config_file);

    let providers = provider::get_providers(&config_file)
        .map_err(|_| AppError::ConfigParseError(config_file))?;

    let mut vars: HashMap<String, String> = HashMap::new();
    match providers.get(provider) {
        Some(selected_provider) => {
            let api_key = cli.get_api_key(provider)?;
            let provider_vars = selected_provider.get_env_vars(api_key);
            vars.extend(provider_vars.into_iter());
        }
        None => {
            return Err(AppError::ProviderNotFound(provider.to_string()));
        }
    }
    _ = launch_claude(vars, &cli.args);
    Ok(())
}
