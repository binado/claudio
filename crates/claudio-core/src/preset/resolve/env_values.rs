use crate::env;
use crate::preset::resolve::command::{ResolverConfig, execute_command};
use crate::preset::resolve::paths::resolve_file_path_for_source;
use crate::preset::types::{EnvValue, EnvValueSource, Preset, PresetSource};
use anyhow::{Context, Result};
use std::collections::HashMap;

/// Resolve environment variables for a preset.
///
/// This signature requires PresetSource for file path resolution.
pub fn resolve_variables_with_source(
    preset: &Preset,
    source: &PresetSource,
    cfg: &ResolverConfig<'_>,
) -> Result<HashMap<String, String>> {
    let mut resolved = HashMap::new();

    if let Some(env_vars) = &preset.env {
        for (key, value) in env_vars {
            let resolved_value = resolve_env_value(key, value, source, cfg)?;
            resolved.insert(key.clone(), resolved_value);
        }
    }

    Ok(resolved)
}

fn resolve_env_value(
    env_key: &str,
    env_value: &EnvValue,
    source: &PresetSource,
    cfg: &ResolverConfig<'_>,
) -> Result<String> {
    match env_value {
        EnvValue::Direct(s) => resolve_direct_value(s),
        EnvValue::Structured {
            source: value_source,
        } => match value_source {
            EnvValueSource::Value { value } => Ok(value.clone()),
            EnvValueSource::Env { var } => env::get_required_env_var(var),
            EnvValueSource::File { path } => {
                let resolved_path = resolve_file_path_for_source(path, source)?;
                let content = std::fs::read_to_string(&resolved_path)
                    .with_context(|| format!("Failed to read file: {}", resolved_path.display()))?;
                Ok(content.trim_end().to_string())
            }
            EnvValueSource::Command { command, confirm } => {
                execute_command(env_key, command, *confirm, cfg)
            }
        },
    }
}

fn resolve_direct_value(value: &str) -> Result<String> {
    env::substitute_env_vars(value)
}
