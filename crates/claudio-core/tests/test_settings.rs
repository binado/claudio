mod common;

use claudio_core::scope::Scope;
use claudio_core::settings::loader as settings_loader;
use common::TestEnvironment;

struct CwdGuard {
    original: std::path::PathBuf,
}

impl CwdGuard {
    fn new(new_dir: &std::path::Path) -> Self {
        let original = std::env::current_dir().expect("Failed to get current dir");
        std::env::set_current_dir(new_dir).expect("Failed to set current dir");
        Self { original }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
}

#[test]
#[serial_test::serial]
fn test_load_settings_from_project() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create settings file
    env.write_project_settings(
        r#"{
  "default_preset": "test-preset",
  "color": "green",
  "no_color": true
}"#,
    );

    // Load settings
    let settings = settings_loader::load_settings(Scope::Project)
        .unwrap()
        .unwrap();

    assert_eq!(settings.default_preset, Some("test-preset".to_string()));
    assert_eq!(settings.color, Some("green".to_string()));
    assert_eq!(settings.no_color, Some(true));

    env.cleanup_env_vars();
}

#[test]
#[serial_test::serial]
fn test_load_settings_from_user() {
    let env = TestEnvironment::new();
    env.create_user_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create settings file
    env.write_user_settings(
        r#"{
  "default_preset": "user-preset",
  "color": "blue",
  "require_preset": true
}"#,
    );

    // Load settings
    let settings = settings_loader::load_settings(Scope::User)
        .unwrap()
        .unwrap();

    assert_eq!(settings.default_preset, Some("user-preset".to_string()));
    assert_eq!(settings.color, Some("blue".to_string()));
    assert_eq!(settings.require_preset, Some(true));

    env.cleanup_env_vars();
}

#[test]
#[serial_test::serial]
fn test_project_settings_shadow_user_settings() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.create_user_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create user settings
    env.write_user_settings(
        r#"{
  "default_preset": "user-preset",
  "color": "blue"
}"#,
    );

    // Create project settings
    env.write_project_settings(
        r#"{
  "default_preset": "project-preset",
  "color": "green"
}"#,
    );

    // Load settings with auto scope (project should take precedence)
    let settings = settings_loader::load_settings(Scope::Auto)
        .unwrap()
        .unwrap();

    assert_eq!(settings.default_preset, Some("project-preset".to_string()));
    assert_eq!(settings.color, Some("green".to_string()));

    env.cleanup_env_vars();
}

#[test]
#[serial_test::serial]
fn test_default_settings_when_file_missing() {
    let env = TestEnvironment::new();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Ensure project root detection works without creating `.claudio`
    std::fs::create_dir_all(env.project_dir.join(".git")).unwrap();

    // Load settings when no file exists
    let settings = settings_loader::load_settings(Scope::Project).unwrap();
    assert!(settings.is_none());

    env.cleanup_env_vars();
}

#[test]
#[serial_test::serial]
fn test_invalid_settings_file() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create invalid JSON settings
    env.write_project_settings("{ invalid json }");

    // Loading should return error
    let result = settings_loader::load_settings(Scope::Project);
    assert!(result.is_err());

    env.cleanup_env_vars();
}

#[test]
#[serial_test::serial]
fn test_load_settings_with_inline_presets() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create settings with inline presets
    env.write_project_settings(
        r#"{
  "presets": [
    {
      "name": "inline-preset",
      "description": "Inline preset",
      "env": {
        "INLINE_VAR": "inline_value"
      }
    }
  ]
}"#,
    );

    // Load settings
    let settings = settings_loader::load_settings(Scope::Project)
        .unwrap()
        .unwrap();

    assert_eq!(settings.presets.len(), 1);
    assert_eq!(settings.presets[0].name, "inline-preset");
    assert_eq!(
        settings.presets[0].description,
        Some("Inline preset".to_string())
    );
    assert_eq!(
        settings.presets[0]
            .env
            .as_ref()
            .and_then(|e| e.get("INLINE_VAR")),
        Some(&claudio_core::preset::types::EnvValue::Direct(
            "inline_value".to_string()
        ))
    );

    env.cleanup_env_vars();
}

#[test]
#[serial_test::serial]
fn test_settings_validation_conflicting_inline_preset_name() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create a file-based preset
    env.write_project_preset(
        "duplicate",
        r#"{
  "name": "duplicate",
  "env": {}
}"#,
    );

    // Create settings with an inline preset using the same name
    env.write_project_settings(
        r#"{
  "presets": [
    {
      "name": "duplicate",
      "env": {}
    }
  ]
}"#,
    );

    // Loading should fail validation due to name conflict with file preset
    let result = settings_loader::load_settings(Scope::Project);
    assert!(result.is_err());

    env.cleanup_env_vars();
}

#[test]
#[serial_test::serial]
fn test_load_settings_with_all_options() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create settings with all options
    env.write_project_settings(
        r#"{
  "default_preset": "test-preset",
  "color": "yellow",
  "no_color": true,
  "require_preset": true,
  "ignore_user_presets": true,
  "presets": []
}"#,
    );

    // Load settings
    let settings = settings_loader::load_settings(Scope::Project)
        .unwrap()
        .unwrap();

    assert_eq!(settings.default_preset, Some("test-preset".to_string()));
    assert_eq!(settings.color, Some("yellow".to_string()));
    assert_eq!(settings.no_color, Some(true));
    assert_eq!(settings.require_preset, Some(true));
    assert_eq!(settings.ignore_user_presets, Some(true));
    assert_eq!(settings.presets.len(), 0);

    env.cleanup_env_vars();
}
