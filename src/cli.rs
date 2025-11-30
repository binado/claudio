use clap::Parser;
use std::env;
use std::path::PathBuf;

use crate::error::{AppError, Result};

#[derive(Parser)]
#[command(name = "cchoose", about = "Change claude code model on the fly")]
pub struct Cli {
    pub provider: Option<String>,
    #[arg(short = 'c', long)]
    pub config_file: Option<PathBuf>,
    #[arg(long)]
    pub api_key: Option<String>,
    #[arg(allow_hyphen_values = true, num_args = 0.., last = true)]
    pub args: Vec<String>,
}

impl Cli {
    pub fn get_api_key(&self, provider: &str) -> Result<String> {
        if let Some(key) = &self.api_key {
            return Ok(key.clone());
        }

        let var_name = format!("{}_API_KEY", provider.to_uppercase());
        env::var(&var_name).map_err(|_| AppError::ApiKeyNotFound(provider.to_string()))
    }
}

pub fn get_cli() -> Cli {
    Cli::parse()
}
