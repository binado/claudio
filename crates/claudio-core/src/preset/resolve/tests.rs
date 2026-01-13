use super::*;
use crate::env::CLAUDIO_HOME_DIR_ENV_VAR;
use serde_json::json;
use serial_test::serial;
use std::collections::{HashMap, HashSet};
use tempfile::tempdir;

use crate::preset::resolve::inheritance::resolve_inheritance_recursive;
use crate::preset::resolve::settings::{merge_json_settings, resolve_settings_value};
use crate::preset::types::{EnvValue, EnvValueSource, Preset, PresetSource};

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
fn test_prompt_normalized_to_none_when_empty_or_whitespace() {
    let preset_whitespace = Preset {
        name: "test".to_string(),
        description: None,
        extends: None,
        prompt: Some("   ".to_string()),
        env: None,
        args: None,
        settings: None,
    };

    let source = PresetSource::Inline;
    let cfg = ResolverConfig::default();

    let resolved_whitespace = resolve_inheritance_recursive(
        &preset_whitespace,
        source.clone(),
        None,
        &cfg,
        &mut HashSet::new(),
        &mut Vec::new(),
    )
    .unwrap();

    assert!(resolved_whitespace.prompt.is_none());

    let preset_empty = Preset {
        name: "test".to_string(),
        description: None,
        extends: None,
        prompt: Some(String::new()),
        env: None,
        args: None,
        settings: None,
    };

    let resolved_empty = resolve_inheritance_recursive(
        &preset_empty,
        source,
        None,
        &cfg,
        &mut HashSet::new(),
        &mut Vec::new(),
    )
    .unwrap();

    assert!(resolved_empty.prompt.is_none());
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
    let source = PresetSource::Inline;
    let cfg = ResolverConfig::default();
    let result = resolve_variables_with_source(&preset, &source, &cfg).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_resolve_variables_no_substitution() {
    let mut env_map = HashMap::new();
    env_map.insert(
        "API_URL".to_string(),
        EnvValue::Direct("https://api.example.com".to_string()),
    );
    let preset = preset_with_env(env_map);
    let source = PresetSource::Inline;

    let cfg = ResolverConfig::default();
    let result = resolve_variables_with_source(&preset, &source, &cfg).unwrap();
    assert_eq!(result.get("API_URL").unwrap(), "https://api.example.com");
}

#[test]
#[serial]
fn test_resolve_variables_single_substitution() {
    unsafe {
        std::env::set_var("TEST_API_KEY", "secret123");
    }

    let mut env_map = HashMap::new();
    env_map.insert(
        "API_KEY".to_string(),
        EnvValue::Direct("${TEST_API_KEY}".to_string()),
    );
    let preset = preset_with_env(env_map);
    let source = PresetSource::Inline;

    let cfg = ResolverConfig::default();
    let result = resolve_variables_with_source(&preset, &source, &cfg).unwrap();
    assert_eq!(result.get("API_KEY").unwrap(), "secret123");

    unsafe {
        std::env::remove_var("TEST_API_KEY");
    }
}

#[test]
#[serial]
fn test_resolve_variables_multiple_substitutions() {
    unsafe {
        std::env::set_var("VAR1", "hello");
        std::env::set_var("VAR2", "world");
    }

    let mut env_map = HashMap::new();
    env_map.insert(
        "COMBINED".to_string(),
        EnvValue::Direct("${VAR1}_${VAR2}".to_string()),
    );
    let preset = preset_with_env(env_map);
    let source = PresetSource::Inline;

    let cfg = ResolverConfig::default();
    let result = resolve_variables_with_source(&preset, &source, &cfg).unwrap();
    assert_eq!(result.get("COMBINED").unwrap(), "hello_world");

    unsafe {
        std::env::remove_var("VAR1");
        std::env::remove_var("VAR2");
    }
}

#[test]
fn test_resolve_variables_missing_env_var() {
    let mut env_map = HashMap::new();
    env_map.insert(
        "API_KEY".to_string(),
        EnvValue::Direct("${NONEXISTENT_VAR_12345}".to_string()),
    );
    let preset = preset_with_env(env_map);
    let source = PresetSource::Inline;

    let cfg = ResolverConfig::default();
    let result = resolve_variables_with_source(&preset, &source, &cfg);
    assert!(result.is_err());

    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("Environment variable 'NONEXISTENT_VAR_12345' is not set"));
    assert!(msg.contains("export NONEXISTENT_VAR_12345=your_value"));
}

#[test]
#[serial]
fn test_resolve_variables_structured_env_source() {
    unsafe {
        std::env::set_var("STRUCTURED_API_KEY", "structured_value");
    }

    let mut env_map = HashMap::new();
    env_map.insert(
        "API_KEY".to_string(),
        EnvValue::Structured {
            source: EnvValueSource::Env {
                var: "STRUCTURED_API_KEY".to_string(),
            },
        },
    );

    let preset = preset_with_env(env_map);
    let source = PresetSource::Inline;
    let cfg = ResolverConfig::default();
    let result = resolve_variables_with_source(&preset, &source, &cfg).unwrap();
    assert_eq!(result.get("API_KEY").unwrap(), "structured_value");

    unsafe {
        std::env::remove_var("STRUCTURED_API_KEY");
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

    let mut env_map = HashMap::new();
    env_map.insert(
        "TOKEN".to_string(),
        EnvValue::Structured {
            source: EnvValueSource::File {
                path: "./secret.txt".to_string(),
            },
        },
    );
    let preset = preset_with_env(env_map);

    let source = PresetSource::File(preset_path);
    let cfg = ResolverConfig::default();
    let result = resolve_variables_with_source(&preset, &source, &cfg).unwrap();
    assert_eq!(result.get("TOKEN").unwrap(), "file_secret");
}

#[test]
#[serial]
fn test_resolve_variables_structured_command_source_success_no_confirm() {
    let command = "echo cmd_value";

    let mut env_map = HashMap::new();
    env_map.insert(
        "FROM_CMD".to_string(),
        EnvValue::Structured {
            source: EnvValueSource::Command {
                command: command.to_string(),
                confirm: Some(false),
            },
        },
    );
    let preset = preset_with_env(env_map);
    let source = PresetSource::Inline;

    let cfg = ResolverConfig::default();
    let result = resolve_variables_with_source(&preset, &source, &cfg).unwrap();
    assert_eq!(result.get("FROM_CMD").unwrap(), "cmd_value");
}

#[test]
#[serial]
fn test_resolve_variables_structured_command_source_failure_includes_stderr() {
    let command = if cfg!(target_os = "windows") {
        "echo cmd_err 1>&2 & exit /B 7"
    } else {
        "echo cmd_err 1>&2; exit 7"
    };

    let mut env_map = HashMap::new();
    env_map.insert(
        "FROM_CMD".to_string(),
        EnvValue::Structured {
            source: EnvValueSource::Command {
                command: command.to_string(),
                confirm: Some(false),
            },
        },
    );
    let preset = preset_with_env(env_map);
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
        std::env::set_var("TEST_ARRAY_VAR", "array_value");
    }

    let input = json!(["${TEST_ARRAY_VAR}", "static"]);
    let result = resolve_settings_value(&input).unwrap();
    assert_eq!(result, json!(["array_value", "static"]));

    unsafe {
        std::env::remove_var("TEST_ARRAY_VAR");
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

    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("Environment variable 'MISSING_SETTINGS_VAR_99999' is not set"));
    assert!(msg.contains("export MISSING_SETTINGS_VAR_99999=your_value"));
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
    assert!(merge_json_settings(None, None).is_none());
}

#[test]
fn test_merge_json_settings_base_only() {
    let base = Some(json!({"a": 1}));
    assert_eq!(merge_json_settings(base.clone(), None), base);
}

#[test]
fn test_merge_json_settings_derived_only() {
    let derived = Some(json!({"a": 1}));
    assert_eq!(merge_json_settings(None, derived.clone()), derived);
}

#[test]
fn test_merge_json_settings_shallow_merge() {
    let base = Some(json!({"a": 1, "b": 2}));
    let derived = Some(json!({"b": 3}));
    let merged = merge_json_settings(base, derived);
    assert_eq!(merged, Some(json!({"a": 1, "b": 3})));
}

#[test]
fn test_merge_json_settings_deep_merge() {
    let base = Some(json!({"obj": {"a": 1, "b": 2}}));
    let derived = Some(json!({"obj": {"b": 3, "c": 4}}));
    let merged = merge_json_settings(base, derived);
    assert_eq!(merged, Some(json!({"obj": {"a": 1, "b": 3, "c": 4}})));
}

#[test]
fn test_merge_json_settings_derived_non_object_overrides() {
    let base = Some(json!({"a": 1}));
    let derived = Some(json!("string"));
    let merged = merge_json_settings(base, derived);
    assert_eq!(merged, Some(json!("string")));
}

#[test]
#[serial]
fn test_tilde_expansion_uses_os_home_not_claudio_home_dir() {
    let temp = tempdir().unwrap();

    unsafe {
        std::env::set_var(CLAUDIO_HOME_DIR_ENV_VAR, temp.path());
    }

    let expected_home = directories::BaseDirs::new()
        .unwrap()
        .home_dir()
        .to_owned()
        .join("tilde_test_file");

    let source = PresetSource::Inline;
    let resolved = resolve_file_path_for_source("~/tilde_test_file", &source).unwrap();

    assert_eq!(resolved, expected_home);
    assert!(!resolved.starts_with(temp.path()));

    unsafe {
        std::env::remove_var(CLAUDIO_HOME_DIR_ENV_VAR);
    }
}
