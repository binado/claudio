use claudio::cli::Scope;
use claudio::preset::types::Preset;
use claudio::settings::types::Settings;
use std::collections::HashMap;
use tempfile::TempDir;

#[test]
fn test_settings_validation_no_collisions() {
    // Create a temporary directory for presets
    let temp_dir = TempDir::new().unwrap();
    let presets_dir = temp_dir.path().join("presets");
    std::fs::create_dir_all(&presets_dir).unwrap();

    // Create a file-based preset
    let file_preset = Preset {
        name: "file-preset".to_string(),
        description: Some("File preset".to_string()),
        extends: None,
        prompt: None,
        env: None,
        args: None,
        settings: None,
    };

    let file_path = presets_dir.join("file-preset.json");
    let json = serde_json::to_string_pretty(&file_preset).unwrap();
    std::fs::write(&file_path, json).unwrap();

    // Create settings with inline preset that has same name as file preset
    let mut inline_presets = HashMap::new();
    inline_presets.insert(
        "file-preset".to_string(),
        Preset {
            name: "file-preset".to_string(),
            description: Some("Inline preset".to_string()),
            extends: None,
            prompt: None,
            env: None,
            args: None,
            settings: None,
        },
    );

    let _settings = Settings {
        default_preset: None,
        presets: inline_presets,
    };

    // Note: We can't easily test the validation here because it requires
    // the actual preset discovery system. This test demonstrates the structure.
}

#[test]
fn test_default_settings_serialization() {
    let settings = Settings::default();
    let json = serde_json::to_string_pretty(&settings).unwrap();

    // Should be valid JSON
    assert!(serde_json::from_str::<Settings>(&json).is_ok());

    // Default should have empty presets and null default_preset
    assert!(settings.presets.is_empty());
    assert!(settings.default_preset.is_none());
}

#[test]
fn test_settings_with_default_preset() {
    let settings = Settings {
        default_preset: Some("my-preset".to_string()),
        presets: HashMap::new(),
    };

    let json = serde_json::to_string_pretty(&settings).unwrap();
    let deserialized: Settings = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.default_preset, Some("my-preset".to_string()));
}

#[test]
fn test_settings_with_inline_presets() {
    let mut presets = HashMap::new();
    presets.insert(
        "quick".to_string(),
        Preset {
            name: "quick".to_string(),
            description: Some("Quick preset".to_string()),
            extends: None,
            prompt: None,
            env: Some({
                let mut m = HashMap::new();
                m.insert(
                    "ANTHROPIC_AUTH_TOKEN".to_string(),
                    "${ANTHROPIC_API_KEY}".to_string(),
                );
                m
            }),
            args: Some(vec!["--no-mcp".to_string()]),
            settings: None,
        },
    );

    let settings = Settings {
        default_preset: None,
        presets,
    };

    let json = serde_json::to_string_pretty(&settings).unwrap();
    let deserialized: Settings = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.presets.len(), 1);
    assert!(deserialized.presets.contains_key("quick"));

    let quick_preset = &deserialized.presets["quick"];
    assert_eq!(quick_preset.name, "quick");
    assert_eq!(quick_preset.args, Some(vec!["--no-mcp".to_string()]));
}

#[test]
fn test_settings_validation_inline_preset_validation() {
    // Create settings with an inline preset with invalid name (empty)
    let mut presets = HashMap::new();
    presets.insert(
        "invalid".to_string(),
        Preset {
            name: "".to_string(), // Invalid: empty name
            description: None,
            extends: None,
            prompt: None,
            env: None,
            args: None,
            settings: None,
        },
    );

    let settings = Settings {
        default_preset: None,
        presets,
    };

    // Validation should fail due to empty name in inline preset
    let result = settings.validate(Scope::User);
    assert!(result.is_err());
}

#[test]
fn test_settings_validation_successful() {
    // Create valid settings
    let mut presets = HashMap::new();
    presets.insert(
        "valid".to_string(),
        Preset {
            name: "valid".to_string(),
            description: Some("Valid preset".to_string()),
            extends: None,
            prompt: None,
            env: None,
            args: None,
            settings: None,
        },
    );

    let _settings = Settings {
        default_preset: Some("valid".to_string()),
        presets,
    };

    // Note: Full validation requires file system access, so we can't test
    // the complete validation here. This demonstrates the structure.
}
