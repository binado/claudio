use crate::preset::loader;
use crate::preset::types::{EnvValue, EnvValueSource, Preset, PresetSource, ResolvedPreset};
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use std::sync::LazyLock;

static VAR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").expect("Invalid regex pattern"));

/// Extract `${VAR_NAME}` references from a string.
///
/// This uses the same pattern as variable substitution in preset env values and
/// settings resolution.
pub fn extract_variable_references(value: &str) -> Vec<String> {
    VAR_REGEX
        .captures_iter(value)
        .map(|caps| caps[1].to_string())
        .collect()
}

/// CLI-provided interface for confirming command execution.
///
/// `claudio-core` does not perform terminal I/O. If a preset contains a
/// command-backed env value and confirmation is required, the caller must
/// provide a `ConfirmCommand` implementation via [`ResolverConfig`].
pub trait ConfirmCommand {
    fn confirm_command(&self, request: ConfirmCommandRequest<'_>) -> Result<bool>;
}

#[derive(Debug, Clone, Copy)]
pub struct ConfirmCommandRequest<'a> {
    pub command: &'a str,
}

#[derive(Clone, Copy, Default)]
pub struct ResolverConfig<'a> {
    /// If true, command-backed env values do not require confirmation unless
    /// the preset explicitly sets `confirm: true`.
    pub skip_command_confirmation: bool,

    /// Confirmation handler.
    ///
    /// If confirmation is required and this is `None`, resolution fails with an error.
    pub confirm_command: Option<&'a dyn ConfirmCommand>,
}

/// Resolve environment variables for a preset
/// This is the new signature that requires PresetSource for file path resolution
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

/// Resolve a single environment value
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
            EnvValueSource::Env { var } => std::env::var(var).with_context(|| {
                format!(
                    "Environment variable '{}' is not set.\n\nSet it with:\n  export {}=your_value",
                    var, var
                )
            }),
            EnvValueSource::File { path } => {
                let resolved_path = resolve_file_path(path, source)?;
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

/// Resolve direct string values (legacy ${VAR} syntax)
fn resolve_direct_value(value: &str) -> Result<String> {
    // Check all variables exist before replacement
    for caps in VAR_REGEX.captures_iter(value) {
        let var_name = &caps[1];
        std::env::var(var_name).with_context(|| {
            format!(
                "Environment variable '{}' is not set.\n\nSet it with:\n  export {}=your_value",
                var_name, var_name
            )
        })?;
    }

    // Now replace (we know all vars exist)
    let resolved_value = VAR_REGEX.replace_all(value, |caps: &regex::Captures| {
        let var_name = &caps[1];
        std::env::var(var_name).expect("Variable should exist (already validated)")
    });

    Ok(resolved_value.to_string())
}

/// Resolve a file path relative to the preset source location
fn resolve_file_path(path: &str, source: &PresetSource) -> Result<PathBuf> {
    // Expand tilde (~) to home directory
    let path = if path == "~" || path.starts_with("~/") {
        let home = directories::BaseDirs::new()
            .context("Could not determine home directory")?
            .home_dir()
            .to_owned();
        home.join(path.strip_prefix("~/").unwrap_or(""))
    } else {
        PathBuf::from(path)
    };

    // If already absolute, use as-is
    if path.is_absolute() {
        return Ok(path);
    }

    // For relative paths, resolve based on preset source
    match source {
        PresetSource::File(preset_path) => {
            // Resolve relative to the preset file's directory
            let preset_dir = preset_path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Invalid preset path: no parent directory"))?;
            Ok(preset_dir.join(path))
        }
        PresetSource::Inline => {
            // For inline presets, resolve from current directory
            Ok(std::env::current_dir()?.join(path))
        }
    }
}

/// Resolve a file path in a preset's env value relative to its source.
///
/// This mirrors the behavior used by env resolution:
/// - `~` expansion
/// - absolute paths kept as-is
/// - relative paths resolved relative to the preset file directory (file presets)
///   or current directory (inline presets)
pub fn resolve_file_path_for_source(path: &str, source: &PresetSource) -> Result<PathBuf> {
    resolve_file_path(path, source)
}

/// Execute a command and return its stdout
///
/// If confirmation is required and [`ResolverConfig::confirm_command`] is `None`,
/// this function returns an error rather than prompting.
fn execute_command(
    env_key: &str,
    command: &str,
    confirm: Option<bool>,
    cfg: &ResolverConfig<'_>,
) -> Result<String> {
    // Determine if we should confirm based on settings and command-specific flag
    let should_confirm = should_confirm_command(confirm, cfg);

    if should_confirm {
        let Some(confirmer) = cfg.confirm_command else {
            anyhow::bail!(
                "Command confirmation required for env key '{}' but no confirmer was provided.
\
\
This is a safety feature: claudio-core does not prompt for input.
Provide a ConfirmCommand implementation (CLI) or set 'skip_command_confirmation' to true in settings.",
                env_key
            );
        };

        let allowed = confirmer.confirm_command(ConfirmCommandRequest { command })?;
        if !allowed {
            anyhow::bail!("Command execution denied");
        }
    }

    // Choose shell per platform to support Windows and Unix-like systems
    let (shell, shell_arg) = if cfg!(target_os = "windows") {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    let output = Command::new(shell)
        .arg(shell_arg)
        .arg(command)
        .output()
        .with_context(|| format!("Failed to execute command: {}", command))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Command failed with exit code {:?}: {}\n\nstderr:\n{}",
            output.status.code(),
            command,
            stderr.trim_end()
        );
    }

    Ok(String::from_utf8(output.stdout)?.trim_end().to_string())
}

/// Determine if command execution should prompt for confirmation
fn should_confirm_command(confirm: Option<bool>, cfg: &ResolverConfig<'_>) -> bool {
    // If confirm is explicitly set on the command, use that.
    if let Some(confirm_value) = confirm {
        return confirm_value;
    }

    // Otherwise use the global policy.
    // Default is secure-by-default: confirmation required.
    !cfg.skip_command_confirmation
}

pub fn resolve_settings_variables(settings: &serde_json::Value) -> Result<serde_json::Value> {
    match settings {
        serde_json::Value::Object(obj) => {
            let mut resolved_map = serde_json::Map::new();
            for (key, value) in obj {
                resolved_map.insert(key.clone(), resolve_settings_value(value)?);
            }
            Ok(serde_json::Value::Object(resolved_map))
        }
        other => {
            let resolved = resolve_settings_value(other)?;
            Ok(resolved)
        }
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
            let mut resolved_map = serde_json::Map::new();
            for (key, val) in obj {
                resolved_map.insert(key.clone(), resolve_settings_value(val)?);
            }
            Ok(serde_json::Value::Object(resolved_map))
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
                let merged_value = if let Some(base_value) = base_obj.get(&key) {
                    merge_json_settings(Some(base_value.clone()), Some(value.clone()))
                        .unwrap_or(value)
                } else {
                    value
                };
                base_obj.insert(key, merged_value);
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

/// Resolve preset inheritance with scope awareness.
///
/// This is the preferred function for resolving presets as it:
/// - Respects the scope when looking up base presets
/// - Properly handles inline presets (no disk lookup for source)
/// - Uses the PresetStore for all lookups
///
/// # Arguments
/// * `preset` - The preset to resolve
/// * `source` - Where the preset came from (Inline or File)
/// * `store` - The PresetStore to use for base preset lookups
pub fn resolve_inheritance_with_store(
    preset: &Preset,
    source: PresetSource,
    store: &crate::preset::store::PresetStore,
    cfg: &ResolverConfig<'_>,
) -> Result<ResolvedPreset> {
    resolve_inheritance_recursive(
        preset,
        source,
        Some(store),
        cfg,
        &mut HashSet::new(),
        &mut Vec::new(),
    )
}

fn resolve_inheritance_recursive(
    preset: &Preset,
    source: PresetSource,
    store: Option<&crate::preset::store::PresetStore>,
    cfg: &ResolverConfig<'_>,
    visited_set: &mut HashSet<String>,
    visited_vec: &mut Vec<String>,
) -> Result<ResolvedPreset> {
    if visited_set.contains(&preset.name) {
        let chain: Vec<String> = visited_vec
            .iter()
            .cloned()
            .chain(std::iter::once(preset.name.clone()))
            .collect();
        anyhow::bail!(
            "Circular inheritance detected: {}\n\nCheck the 'extends' field in these presets for a circular dependency.",
            chain.join(" -> ")
        );
    }

    visited_set.insert(preset.name.clone());
    visited_vec.push(preset.name.clone());

    let mut env_vars = resolve_variables_with_source(preset, &source, cfg)?;
    let args;
    let settings;

    if let Some(base_name) = &preset.extends {
        // Look up base preset - use store if available (scope-aware), otherwise use loader
        let (base_preset, base_source) = if let Some(s) = store {
            // Scope-aware lookup via PresetStore
            let located = s
                .find_required(base_name)
                .with_context(|| format!("Failed to find base preset: {}", base_name))?;
            (located.preset, located.source)
        } else {
            // Fallback: unscoped lookup via loader (backward compatibility)
            let base_path = loader::find_preset(base_name)
                .with_context(|| format!("Failed to find base preset: {}", base_name))?;
            let base_preset = loader::load_preset(&base_path)
                .with_context(|| format!("Failed to load base preset: {}", base_name))?;
            (base_preset, PresetSource::File(base_path))
        };

        let base_resolved = resolve_inheritance_recursive(
            &base_preset,
            base_source,
            store,
            cfg,
            visited_set,
            visited_vec,
        )?;

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
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serial_test::serial;
    use tempfile::tempdir;

    // ─────────────────────────────────────────────────────────────────────────────
    // resolve_variables tests
    // ─────────────────────────────────────────────────────────────────────────────

    fn preset_with_env(env: HashMap<String, EnvValue>) -> Preset {
        Preset {
            name: "test".to_string(),
            description: None,
            extends: None,
            prompt: None,
            env: Some(env),
            args: None,
            settings: None,
        }
    }

    #[test]
    fn test_resolve_variables_no_env() {
        let preset = Preset {
            name: "test".to_string(),
            description: None,
            extends: None,
            prompt: None,
            env: None,
            args: None,
            settings: None,
        };
        let source = PresetSource::Inline; // Use Inline since we don't have a real file
        let cfg = ResolverConfig::default();
        let result = resolve_variables_with_source(&preset, &source, &cfg).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_resolve_variables_no_substitution() {
        let mut env = HashMap::new();
        env.insert(
            "API_URL".to_string(),
            EnvValue::Direct("https://api.example.com".to_string()),
        );
        let preset = preset_with_env(env);
        let source = PresetSource::Inline;

        let cfg = ResolverConfig::default();
        let result = resolve_variables_with_source(&preset, &source, &cfg).unwrap();
        assert_eq!(result.get("API_URL").unwrap(), "https://api.example.com");
    }

    #[test]
    #[serial]
    fn test_resolve_variables_single_substitution() {
        // Set up test environment variable
        unsafe {
            std::env::set_var("TEST_API_KEY", "secret123");
        }

        let mut env = HashMap::new();
        env.insert(
            "API_KEY".to_string(),
            EnvValue::Direct("${TEST_API_KEY}".to_string()),
        );
        let preset = preset_with_env(env);
        let source = PresetSource::Inline;

        let cfg = ResolverConfig::default();
        let result = resolve_variables_with_source(&preset, &source, &cfg).unwrap();
        assert_eq!(result.get("API_KEY").unwrap(), "secret123");

        // Clean up
        unsafe {
            std::env::remove_var("TEST_API_KEY");
        }
    }

    #[test]
    #[serial]
    fn test_resolve_variables_multiple_substitutions() {
        unsafe {
            std::env::set_var("TEST_HOST", "example.com");
            std::env::set_var("TEST_PORT", "8080");
        }

        let mut env = HashMap::new();
        env.insert(
            "API_URL".to_string(),
            EnvValue::Direct("https://${TEST_HOST}:${TEST_PORT}/api".to_string()),
        );
        let preset = preset_with_env(env);
        let source = PresetSource::Inline;

        let cfg = ResolverConfig::default();
        let result = resolve_variables_with_source(&preset, &source, &cfg).unwrap();
        assert_eq!(
            result.get("API_URL").unwrap(),
            "https://example.com:8080/api"
        );

        unsafe {
            std::env::remove_var("TEST_HOST");
            std::env::remove_var("TEST_PORT");
        }
    }

    #[test]
    fn test_resolve_variables_missing_env_var() {
        let mut env = HashMap::new();
        env.insert(
            "API_KEY".to_string(),
            EnvValue::Direct("${NONEXISTENT_VAR_12345}".to_string()),
        );
        let preset = preset_with_env(env);
        let source = PresetSource::Inline;

        let cfg = ResolverConfig::default();
        let result = resolve_variables_with_source(&preset, &source, &cfg);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("NONEXISTENT_VAR_12345")
        );
    }

    #[test]
    #[serial]
    fn test_resolve_variables_structured_env_source() {
        unsafe {
            std::env::set_var("TEST_STRUCTURED_ENV_VAR", "structured_value");
        }

        let mut env = HashMap::new();
        env.insert(
            "API_KEY".to_string(),
            EnvValue::Structured {
                source: EnvValueSource::Env {
                    var: "TEST_STRUCTURED_ENV_VAR".to_string(),
                },
            },
        );

        let preset = preset_with_env(env);
        let source = PresetSource::Inline;
        let cfg = ResolverConfig::default();
        let result = resolve_variables_with_source(&preset, &source, &cfg).unwrap();
        assert_eq!(result.get("API_KEY").unwrap(), "structured_value");

        unsafe {
            std::env::remove_var("TEST_STRUCTURED_ENV_VAR");
        }
    }

    #[test]
    #[serial]
    fn test_resolve_variables_structured_file_source_relative_to_preset_file() {
        let dir = tempdir().unwrap();
        let preset_path = dir.path().join("preset.json");
        std::fs::write(&preset_path, "{}\n").unwrap();

        let secret_path = dir.path().join("secret.txt");
        std::fs::write(&secret_path, "file_secret\n").unwrap();

        let mut env = HashMap::new();
        env.insert(
            "TOKEN".to_string(),
            EnvValue::Structured {
                source: EnvValueSource::File {
                    path: "secret.txt".to_string(),
                },
            },
        );
        let preset = preset_with_env(env);

        let source = PresetSource::File(preset_path);
        let cfg = ResolverConfig::default();
        let result = resolve_variables_with_source(&preset, &source, &cfg).unwrap();
        assert_eq!(result.get("TOKEN").unwrap(), "file_secret");
    }

    #[test]
    #[serial]
    fn test_resolve_variables_structured_command_source_success_no_confirm() {
        let command = if cfg!(target_os = "windows") {
            "echo cmd_value"
        } else {
            "printf cmd_value"
        };

        let mut env = HashMap::new();
        env.insert(
            "FROM_CMD".to_string(),
            EnvValue::Structured {
                source: EnvValueSource::Command {
                    command: command.to_string(),
                    confirm: Some(false),
                },
            },
        );
        let preset = preset_with_env(env);
        let source = PresetSource::Inline;

        let cfg = ResolverConfig::default();
        let result = resolve_variables_with_source(&preset, &source, &cfg).unwrap();
        assert_eq!(result.get("FROM_CMD").unwrap(), "cmd_value");
    }

    #[test]
    #[serial]
    fn test_resolve_variables_structured_command_source_failure_includes_stderr() {
        let command = if cfg!(target_os = "windows") {
            // Write to stderr and exit non-zero
            "echo cmd_err 1>&2 & exit /B 7"
        } else {
            "echo cmd_err 1>&2; exit 7"
        };

        let mut env = HashMap::new();
        env.insert(
            "FROM_CMD".to_string(),
            EnvValue::Structured {
                source: EnvValueSource::Command {
                    command: command.to_string(),
                    confirm: Some(false),
                },
            },
        );
        let preset = preset_with_env(env);
        let source = PresetSource::Inline;

        let cfg = ResolverConfig::default();
        let err = resolve_variables_with_source(&preset, &source, &cfg).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("stderr:"));
        assert!(msg.contains("cmd_err"));
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // resolve_settings_value / resolve_settings_variables tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_resolve_settings_value_primitives() {
        // Numbers, bools, and nulls should pass through unchanged
        assert_eq!(resolve_settings_value(&json!(42)).unwrap(), json!(42));
        assert_eq!(resolve_settings_value(&json!(3.15)).unwrap(), json!(3.15));
        assert_eq!(resolve_settings_value(&json!(true)).unwrap(), json!(true));
        assert_eq!(resolve_settings_value(&json!(null)).unwrap(), json!(null));
    }

    #[test]
    fn test_resolve_settings_value_string_no_vars() {
        let result = resolve_settings_value(&json!("plain string")).unwrap();
        assert_eq!(result, json!("plain string"));
    }

    #[test]
    #[serial]
    fn test_resolve_settings_value_string_with_var() {
        unsafe {
            std::env::set_var("TEST_SETTING_VAR", "resolved_value");
        }

        let result = resolve_settings_value(&json!("prefix_${TEST_SETTING_VAR}_suffix")).unwrap();
        assert_eq!(result, json!("prefix_resolved_value_suffix"));

        unsafe {
            std::env::remove_var("TEST_SETTING_VAR");
        }
    }

    #[test]
    #[serial]
    fn test_resolve_settings_value_array() {
        unsafe {
            std::env::set_var("TEST_ARR_VAR", "item2");
        }

        let input = json!(["item1", "${TEST_ARR_VAR}", "item3"]);
        let result = resolve_settings_value(&input).unwrap();
        assert_eq!(result, json!(["item1", "item2", "item3"]));

        unsafe {
            std::env::remove_var("TEST_ARR_VAR");
        }
    }

    #[test]
    #[serial]
    fn test_resolve_settings_value_nested_object() {
        unsafe {
            std::env::set_var("TEST_NESTED_VAR", "deep_value");
        }

        let input = json!({
            "level1": {
                "level2": "${TEST_NESTED_VAR}"
            }
        });
        let result = resolve_settings_value(&input).unwrap();
        assert_eq!(
            result,
            json!({
                "level1": {
                    "level2": "deep_value"
                }
            })
        );

        unsafe {
            std::env::remove_var("TEST_NESTED_VAR");
        }
    }

    #[test]
    fn test_resolve_settings_value_missing_var() {
        let input = json!("${MISSING_SETTINGS_VAR_99999}");
        let result = resolve_settings_value(&input);
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_resolve_settings_variables_object() {
        unsafe {
            std::env::set_var("TEST_OBJ_VAR", "obj_value");
        }

        let input = json!({
            "key1": "static",
            "key2": "${TEST_OBJ_VAR}"
        });
        let result = resolve_settings_variables(&input).unwrap();
        assert_eq!(
            result,
            json!({
                "key1": "static",
                "key2": "obj_value"
            })
        );

        unsafe {
            std::env::remove_var("TEST_OBJ_VAR");
        }
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // merge_json_settings tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_merge_json_settings_both_none() {
        let result = merge_json_settings(None, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_merge_json_settings_base_only() {
        let base = json!({"key": "value"});
        let result = merge_json_settings(Some(base.clone()), None);
        assert_eq!(result, Some(base));
    }

    #[test]
    fn test_merge_json_settings_derived_only() {
        let derived = json!({"key": "value"});
        let result = merge_json_settings(None, Some(derived.clone()));
        assert_eq!(result, Some(derived));
    }

    #[test]
    fn test_merge_json_settings_shallow_merge() {
        let base = json!({"a": 1, "b": 2});
        let derived = json!({"b": 3, "c": 4});
        let result = merge_json_settings(Some(base), Some(derived)).unwrap();

        assert_eq!(result["a"], json!(1));
        assert_eq!(result["b"], json!(3)); // derived overrides
        assert_eq!(result["c"], json!(4));
    }

    #[test]
    fn test_merge_json_settings_deep_merge() {
        let base = json!({
            "nested": {
                "a": 1,
                "b": 2
            }
        });
        let derived = json!({
            "nested": {
                "b": 3,
                "c": 4
            }
        });
        let result = merge_json_settings(Some(base), Some(derived)).unwrap();

        assert_eq!(result["nested"]["a"], json!(1));
        assert_eq!(result["nested"]["b"], json!(3)); // derived overrides
        assert_eq!(result["nested"]["c"], json!(4));
    }

    #[test]
    fn test_merge_json_settings_derived_non_object_overrides() {
        let base = json!({"key": "value"});
        let derived = json!("string value");
        let result = merge_json_settings(Some(base), Some(derived));
        assert_eq!(result, Some(json!("string value")));
    }
}
