use crate::preset::loader;
use crate::preset::types::{Preset, ResolvedPreset};
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

static VAR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").expect("Invalid regex pattern"));

pub fn resolve_variables(preset: &Preset) -> Result<HashMap<String, String>> {
    let mut resolved = HashMap::new();

    if let Some(env_vars) = &preset.env {
        for (key, value) in env_vars {
            // Check all variables exist before replacement
            for caps in VAR_REGEX.captures_iter(value) {
                let var_name = &caps[1];
                std::env::var(var_name)
                    .with_context(|| format!("Environment variable '{}' not found", var_name))?;
            }

            // Now replace (we know all vars exist)
            let resolved_value = VAR_REGEX.replace_all(value, |caps: &regex::Captures| {
                let var_name = &caps[1];
                std::env::var(var_name).expect("Variable should exist (already validated)")
            });
            resolved.insert(key.clone(), resolved_value.to_string());
        }
    }

    Ok(resolved)
}

pub fn resolve_settings_variables(settings: &serde_json::Value) -> Result<serde_json::Value> {
    match settings {
        serde_json::Value::Object(obj) => {
            let mut resolved_obj = serde_json::json!({});
            for (key, value) in obj {
                resolved_obj[key] = resolve_settings_value(value)?;
            }
            Ok(resolved_obj)
        }
        other => Ok(other.clone()),
    }
}

fn resolve_settings_value(value: &serde_json::Value) -> Result<serde_json::Value> {
    match value {
        serde_json::Value::String(s) => {
            // Check all variables exist before replacement
            for caps in VAR_REGEX.captures_iter(s) {
                let var_name = &caps[1];
                std::env::var(var_name)
                    .with_context(|| format!("Environment variable '{}' not found", var_name))?;
            }

            // Now replace (we know all vars exist)
            let resolved_value = VAR_REGEX.replace_all(s, |caps: &regex::Captures| {
                let var_name = &caps[1];
                std::env::var(var_name).expect("Variable should exist (already validated)")
            });
            Ok(serde_json::Value::String(resolved_value.to_string()))
        }
        serde_json::Value::Array(arr) => {
            let resolved: Result<Vec<_>> = arr.iter().map(resolve_settings_value).collect();
            Ok(serde_json::Value::Array(resolved?))
        }
        serde_json::Value::Object(obj) => {
            let mut resolved_obj = serde_json::json!({});
            for (key, val) in obj {
                resolved_obj[key] = resolve_settings_value(val)?;
            }
            Ok(resolved_obj)
        }
        other => Ok(other.clone()),
    }
}

fn merge_json_settings(
    base: Option<serde_json::Value>,
    derived: Option<serde_json::Value>,
) -> Option<serde_json::Value> {
    match (base, derived) {
        (
            Some(serde_json::Value::Object(mut base_obj)),
            Some(serde_json::Value::Object(derived_obj)),
        ) => {
            // Deep merge: derived overrides base
            for (key, value) in derived_obj {
                base_obj.insert(key, value);
            }
            Some(serde_json::Value::Object(base_obj))
        }
        (None, derived) => derived,
        (_base, Some(derived)) => {
            // If derived is present, it takes precedence
            Some(derived)
        }
        (base, None) => base,
    }
}

pub fn resolve_inheritance(preset: &Preset) -> Result<ResolvedPreset> {
    resolve_inheritance_recursive(preset, &mut HashSet::new(), &mut Vec::new())
}

fn resolve_inheritance_recursive(
    preset: &Preset,
    visited_set: &mut HashSet<String>,
    visited_vec: &mut Vec<String>,
) -> Result<ResolvedPreset> {
    if visited_set.contains(&preset.name) {
        let chain: Vec<String> = visited_vec
            .iter()
            .cloned()
            .chain(std::iter::once(preset.name.clone()))
            .collect();
        anyhow::bail!("Circular inheritance detected: {}", chain.join(" -> "));
    }

    visited_set.insert(preset.name.clone());
    visited_vec.push(preset.name.clone());

    let mut env_vars = resolve_variables(preset)?;
    let args;
    let settings;

    if let Some(base_name) = &preset.extends {
        let base_path = loader::find_preset(base_name)
            .with_context(|| format!("Could not find base preset: {}", base_name))?;
        let base_preset = loader::load_preset(&base_path)
            .with_context(|| format!("Failed to load base preset: {}", base_name))?;

        let base_resolved = resolve_inheritance_recursive(&base_preset, visited_set, visited_vec)?;

        for (key, value) in base_resolved.env {
            env_vars.entry(key).or_insert(value);
        }

        // Efficiently extend base args with current preset's args
        let mut merged_args = base_resolved.args;
        if let Some(preset_args) = &preset.args {
            merged_args.extend_from_slice(preset_args);
        }
        args = merged_args;

        // Merge settings: base settings + current preset's settings
        settings = merge_json_settings(base_resolved.settings, preset.settings.clone());
    } else {
        // No inheritance, just use preset's args and settings
        args = preset.args.clone().unwrap_or_default();
        settings = preset.settings.clone();
    }

    visited_set.remove(&preset.name);

    let source_path = loader::find_preset(&preset.name)
        .with_context(|| format!("Could not find preset source path: {}", preset.name))?;

    // Resolve variable substitutions in settings
    let resolved_settings = if let Some(s) = settings {
        Some(resolve_settings_variables(&s)?)
    } else {
        None
    };

    Ok(ResolvedPreset {
        name: preset.name.clone(),
        description: preset.description.clone(),
        prompt: preset.prompt.clone(),
        env: env_vars,
        args,
        settings: resolved_settings,
        source_path,
    })
}
