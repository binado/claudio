//! Integration tests for preset inheritance and variable resolution.
//!
//! Note: Full inheritance chain tests are complex due to the application's
//! search path logic (project + user directories). The core merge/inheritance
//! logic is tested in the unit tests within resolver.rs. These integration tests
//! focus on testing the simpler variable resolution functionality.

mod common;

use claudio::preset::loader;
use claudio::preset::resolver;
use claudio::preset::types::Preset;
use common::TestEnvironment;
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// Variable Resolution Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_resolve_preset_with_env_substitution() {
    // Set up environment variable for test
    unsafe {
        std::env::set_var("TEST_RESOLUTION_API_KEY", "my-secret-key");
    }

    let env = TestEnvironment::new();

    let preset_json = r#"{
        "name": "env-test",
        "env": {
            "API_KEY": "${TEST_RESOLUTION_API_KEY}",
            "STATIC_VAR": "static-value"
        }
    }"#;

    let path = env.create_preset("user", "env-test", preset_json);

    let preset = loader::load_preset(&path).expect("Failed to load preset");
    let resolved = resolver::resolve_variables(&preset).expect("Failed to resolve variables");

    assert_eq!(resolved.get("API_KEY").unwrap(), "my-secret-key");
    assert_eq!(resolved.get("STATIC_VAR").unwrap(), "static-value");

    // Clean up
    unsafe {
        std::env::remove_var("TEST_RESOLUTION_API_KEY");
    }
}

#[test]
fn test_resolve_preset_missing_env_var() {
    let env = TestEnvironment::new();

    let preset_json = r#"{
        "name": "missing-env",
        "env": {
            "API_KEY": "${DEFINITELY_NONEXISTENT_VAR_12345}"
        }
    }"#;

    let path = env.create_preset("user", "missing-env", preset_json);
    let preset = loader::load_preset(&path).expect("Failed to load preset");

    let result = resolver::resolve_variables(&preset);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("DEFINITELY_NONEXISTENT_VAR_12345"));
}

#[test]
fn test_resolve_multiple_vars_in_single_value() {
    unsafe {
        std::env::set_var("TEST_MULTI_HOST", "example.com");
        std::env::set_var("TEST_MULTI_PORT", "8080");
    }

    let env = TestEnvironment::new();

    let preset_json = r#"{
        "name": "multi-var",
        "env": {
            "API_URL": "https://${TEST_MULTI_HOST}:${TEST_MULTI_PORT}/api"
        }
    }"#;

    let path = env.create_preset("user", "multi-var", preset_json);
    let preset = loader::load_preset(&path).expect("Failed to load preset");
    let resolved = resolver::resolve_variables(&preset).expect("Failed to resolve");

    assert_eq!(
        resolved.get("API_URL").unwrap(),
        "https://example.com:8080/api"
    );

    unsafe {
        std::env::remove_var("TEST_MULTI_HOST");
        std::env::remove_var("TEST_MULTI_PORT");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Settings Variable Resolution Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_resolve_settings_with_variables() {
    unsafe {
        std::env::set_var("TEST_SETTINGS_VAR", "resolved-value");
    }

    let settings = serde_json::json!({
        "model": "sonnet",
        "custom_setting": "${TEST_SETTINGS_VAR}"
    });

    let result = resolver::resolve_settings_variables(&settings).expect("Failed to resolve");

    assert_eq!(result["model"], "sonnet");
    assert_eq!(result["custom_setting"], "resolved-value");

    unsafe {
        std::env::remove_var("TEST_SETTINGS_VAR");
    }
}

#[test]
fn test_resolve_nested_settings_with_variables() {
    unsafe {
        std::env::set_var("TEST_NESTED_SETTING", "nested-value");
    }

    let settings = serde_json::json!({
        "outer": {
            "inner": "${TEST_NESTED_SETTING}"
        }
    });

    let result = resolver::resolve_settings_variables(&settings).expect("Failed to resolve");

    assert_eq!(result["outer"]["inner"], "nested-value");

    unsafe {
        std::env::remove_var("TEST_NESTED_SETTING");
    }
}

#[test]
fn test_resolve_array_settings_with_variables() {
    unsafe {
        std::env::set_var("TEST_ARRAY_VAR", "array-value");
    }

    let settings = serde_json::json!({
        "allowed_tools": ["Bash", "${TEST_ARRAY_VAR}", "Read"]
    });

    let result = resolver::resolve_settings_variables(&settings).expect("Failed to resolve");

    let tools = result["allowed_tools"].as_array().unwrap();
    assert_eq!(tools[0], "Bash");
    assert_eq!(tools[1], "array-value");
    assert_eq!(tools[2], "Read");

    unsafe {
        std::env::remove_var("TEST_ARRAY_VAR");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Preset Validation During Load Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_loaded_preset_validation() {
    let env = TestEnvironment::new();

    // Valid preset
    let preset_json = r#"{
        "name": "valid-preset",
        "description": "A valid description",
        "env": {
            "VALID_KEY": "valid-value"
        }
    }"#;

    let path = env.create_preset("user", "valid-preset", preset_json);
    let preset = loader::load_preset(&path).expect("Failed to load preset");

    let validation = preset.validate(Some(&path));
    assert!(validation.is_valid());
    assert!(validation.warnings.is_empty());
}

#[test]
fn test_loaded_preset_with_name_mismatch_warning() {
    let env = TestEnvironment::new();

    let preset_json = r#"{
        "name": "different-name",
        "description": "Name doesn't match filename"
    }"#;

    let path = env.create_preset("user", "actual-filename", preset_json);
    let preset = loader::load_preset(&path).expect("Failed to load preset");

    let validation = preset.validate(Some(&path));
    assert!(validation.is_valid()); // Mismatch is a warning, not an error
    assert!(!validation.warnings.is_empty());
    assert!(
        validation
            .warnings
            .iter()
            .any(|w| w.contains("does not match filename"))
    );
}

#[test]
fn test_loaded_preset_with_empty_prompt_warning() {
    let env = TestEnvironment::new();

    let preset_json = r#"{
        "name": "empty-prompt",
        "prompt": ""
    }"#;

    let path = env.create_preset("user", "empty-prompt", preset_json);
    let preset = loader::load_preset(&path).expect("Failed to load preset");

    let validation = preset.validate(Some(&path));
    assert!(validation.is_valid());
    assert!(
        validation
            .warnings
            .iter()
            .any(|w| w.contains("prompt is empty"))
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Error Case Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_missing_required_env_var_provides_helpful_message() {
    let env = TestEnvironment::new();

    let preset_json = r#"{
        "name": "needs-var",
        "env": {
            "API_KEY": "${MISSING_REQUIRED_VAR_XYZ}"
        }
    }"#;

    let path = env.create_preset("user", "needs-var", preset_json);
    let preset = loader::load_preset(&path).expect("Failed to load preset");

    let result = resolver::resolve_variables(&preset);
    assert!(result.is_err());

    let err = result.unwrap_err().to_string();
    // Should mention the variable name
    assert!(err.contains("MISSING_REQUIRED_VAR_XYZ"));
    // Should suggest how to fix it
    assert!(err.contains("export"));
}
