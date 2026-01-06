//! Integration tests for settings loading and scope precedence.

mod common;

use claudio::cli::Scope;
use claudio::settings::loader as settings_loader;
use claudio::settings::types::Settings;
use common::TestEnvironment;

// ─────────────────────────────────────────────────────────────────────────────
// Settings Loading Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_load_empty_settings() {
    let env = TestEnvironment::new();

    let settings_json = r#"{}"#;
    env.create_settings("user", settings_json);

    unsafe {
        std::env::set_var("CLAUDIO_HOME_DIR", env.home_path());
    }

    let result = settings_loader::load_settings(Scope::User);
    assert!(result.is_ok());

    let settings = result.unwrap();
    assert!(settings.is_some());
    let settings = settings.unwrap();
    assert!(settings.default_preset.is_none());
    assert!(settings.presets.is_empty());

    unsafe {
        std::env::remove_var("CLAUDIO_HOME_DIR");
    }
}

#[test]
fn test_load_settings_with_default_preset() {
    let env = TestEnvironment::new();

    let settings_json = r#"{
        "default_preset": "my-default"
    }"#;
    env.create_settings("user", settings_json);

    unsafe {
        std::env::set_var("CLAUDIO_HOME_DIR", env.home_path());
    }

    let settings = settings_loader::load_settings(Scope::User)
        .expect("Failed to load settings")
        .expect("Settings should exist");

    assert_eq!(settings.default_preset, Some("my-default".to_string()));

    unsafe {
        std::env::remove_var("CLAUDIO_HOME_DIR");
    }
}

#[test]
fn test_load_settings_with_all_options() {
    let env = TestEnvironment::new();

    let settings_json = r#"{
        "default_preset": "default",
        "color": "green",
        "no_color": false,
        "require_preset": true,
        "ignore_user_presets": false,
        "presets": []
    }"#;
    env.create_settings("user", settings_json);

    unsafe {
        std::env::set_var("CLAUDIO_HOME_DIR", env.home_path());
    }

    let settings = settings_loader::load_settings(Scope::User)
        .expect("Failed to load settings")
        .expect("Settings should exist");

    assert_eq!(settings.default_preset, Some("default".to_string()));
    assert_eq!(settings.color, Some("green".to_string()));
    assert_eq!(settings.no_color, Some(false));
    assert_eq!(settings.require_preset, Some(true));
    assert_eq!(settings.ignore_user_presets, Some(false));

    unsafe {
        std::env::remove_var("CLAUDIO_HOME_DIR");
    }
}

#[test]
fn test_load_nonexistent_settings() {
    let env = TestEnvironment::new();
    // Don't create any settings file

    unsafe {
        std::env::set_var("CLAUDIO_HOME_DIR", env.home_path());
    }

    let result = settings_loader::load_settings(Scope::User);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none()); // Should return None, not error

    unsafe {
        std::env::remove_var("CLAUDIO_HOME_DIR");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Inline Preset Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_load_settings_with_inline_presets() {
    let env = TestEnvironment::new();

    let settings_json = r#"{
        "presets": [
            {
                "name": "inline-preset",
                "description": "An inline preset",
                "env": {
                    "API_KEY": "inline-key"
                }
            }
        ]
    }"#;
    env.create_settings("user", settings_json);

    unsafe {
        std::env::set_var("CLAUDIO_HOME_DIR", env.home_path());
    }

    let settings = settings_loader::load_settings(Scope::User)
        .expect("Failed to load settings")
        .expect("Settings should exist");

    assert_eq!(settings.presets.len(), 1);
    assert_eq!(settings.presets[0].name, "inline-preset");

    unsafe {
        std::env::remove_var("CLAUDIO_HOME_DIR");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Scope Precedence Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_resolve_default_preset_user_only() {
    let env = TestEnvironment::new();

    let settings_json = r#"{
        "default_preset": "user-default"
    }"#;
    env.create_settings("user", settings_json);

    unsafe {
        std::env::set_var("CLAUDIO_HOME_DIR", env.home_path());
    }

    let result = settings_loader::resolve_default_preset(Scope::User);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some("user-default".to_string()));

    unsafe {
        std::env::remove_var("CLAUDIO_HOME_DIR");
    }
}

#[test]
fn test_resolve_color_setting() {
    let env = TestEnvironment::new();

    let settings_json = r#"{
        "color": "magenta"
    }"#;
    env.create_settings("user", settings_json);

    unsafe {
        std::env::set_var("CLAUDIO_HOME_DIR", env.home_path());
    }

    let result = settings_loader::resolve_color(Scope::User);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some("magenta".to_string()));

    unsafe {
        std::env::remove_var("CLAUDIO_HOME_DIR");
    }
}

#[test]
fn test_resolve_require_preset_setting() {
    let env = TestEnvironment::new();

    let settings_json = r#"{
        "require_preset": true
    }"#;
    env.create_settings("user", settings_json);

    unsafe {
        std::env::set_var("CLAUDIO_HOME_DIR", env.home_path());
    }

    let result = settings_loader::resolve_require_preset(Scope::User);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some(true));

    unsafe {
        std::env::remove_var("CLAUDIO_HOME_DIR");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Settings Validation Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_settings_validate_empty_presets() {
    let settings = Settings::default();
    let result = settings.validate(Scope::User);
    assert!(result.is_ok());
}

#[test]
fn test_settings_validate_invalid_inline_preset() {
    let env = TestEnvironment::new();

    // Create settings with an invalid inline preset (empty name)
    let settings_json = r#"{
        "presets": [
            {
                "name": "",
                "description": "Invalid preset"
            }
        ]
    }"#;
    env.create_settings("user", settings_json);

    unsafe {
        std::env::set_var("CLAUDIO_HOME_DIR", env.home_path());
    }

    // Loading should fail because validation fails
    let result = settings_loader::load_settings(Scope::User);
    assert!(result.is_err());

    unsafe {
        std::env::remove_var("CLAUDIO_HOME_DIR");
    }
}
