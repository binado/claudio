mod common;

use claudio_core::preset::types::EnvValue;
use claudio_core::preset::{discovery, io, lookup};
use claudio_core::scope::Scope;
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
fn test_load_preset_from_project() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create test preset
    env.write_project_preset(
        "test-preset",
        r#"{
  "name": "test-preset",
  "description": "Test preset",
  "env": {
    "TEST_VAR": "test_value"
  }
}"#,
    );

    // Load preset
    let path = lookup::find_preset_scoped("test-preset", Scope::Project).unwrap();
    let preset = io::load_preset(&path).unwrap();
    assert_eq!(preset.name, "test-preset");
    assert_eq!(preset.description, Some("Test preset".to_string()));
    assert_eq!(
        preset.env.as_ref().and_then(|env| env.get("TEST_VAR")),
        Some(&EnvValue::Direct("test_value".to_string()))
    );

    env.cleanup_env_vars();
}

#[test]
#[serial_test::serial]
fn test_load_preset_from_user() {
    let env = TestEnvironment::new();
    env.create_user_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create test preset
    env.write_user_preset(
        "test-preset",
        r#"{
  "name": "test-preset",
  "description": "Test preset",
  "env": {
    "TEST_VAR": "test_value"
  }
}"#,
    );

    // Load preset
    let path = lookup::find_preset_scoped("test-preset", Scope::User).unwrap();
    let preset = io::load_preset(&path).unwrap();
    assert_eq!(preset.name, "test-preset");
    assert_eq!(preset.description, Some("Test preset".to_string()));
    assert_eq!(
        preset.env.as_ref().and_then(|env| env.get("TEST_VAR")),
        Some(&EnvValue::Direct("test_value".to_string()))
    );

    env.cleanup_env_vars();
}

#[test]
#[serial_test::serial]
fn test_project_preset_shadows_user_preset() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.create_user_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create user preset
    env.write_user_preset(
        "test-preset",
        r#"{
  "name": "test-preset",
  "description": "User preset",
  "env": {
    "TEST_VAR": "user_value"
  }
}"#,
    );

    // Create project preset with same name
    env.write_project_preset(
        "test-preset",
        r#"{
  "name": "test-preset",
  "description": "Project preset",
  "env": {
    "TEST_VAR": "project_value"
  }
}"#,
    );

    // Auto scope should resolve to the project preset path first
    let path = lookup::find_preset_scoped("test-preset", Scope::Auto).unwrap();
    let preset = io::load_preset(&path).unwrap();
    assert_eq!(preset.description, Some("Project preset".to_string()));
    assert_eq!(
        preset.env.as_ref().and_then(|env| env.get("TEST_VAR")),
        Some(&EnvValue::Direct("project_value".to_string()))
    );

    env.cleanup_env_vars();
}

#[test]
#[serial_test::serial]
fn test_load_multiple_presets() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create multiple presets
    env.write_project_preset(
        "preset1",
        r#"{
  "name": "preset1",
  "description": "First preset",
  "env": {
    "VAR1": "value1"
  }
}"#,
    );

    env.write_project_preset(
        "preset2",
        r#"{
  "name": "preset2",
  "description": "Second preset",
  "env": {
    "VAR2": "value2"
  }
}"#,
    );

    // Discover + load presets
    let paths = discovery::discover_presets_scoped(Scope::Project).unwrap();
    assert_eq!(paths.len(), 2);

    let mut names: Vec<String> = paths
        .iter()
        .map(|p| io::load_preset(p).unwrap().name)
        .collect();
    names.sort();
    assert_eq!(names, vec!["preset1".to_string(), "preset2".to_string()]);
}

#[test]
#[serial_test::serial]
fn test_invalid_json_preset_file() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create invalid JSON preset
    env.write_project_preset("invalid", "{ invalid json }");

    // Loading should return error
    let path = env
        .project_dir
        .join(".claudio")
        .join("presets")
        .join("invalid.json");
    let result = io::load_preset(&path);
    assert!(result.is_err());
}

#[test]
#[serial_test::serial]
fn test_missing_presets_directory() {
    let env = TestEnvironment::new();

    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Ensure project root detection works for `Scope::Project`
    std::fs::create_dir_all(env.project_dir.join(".git")).unwrap();

    // Don't create config directory

    // Discover should return empty list for project scope
    let paths = discovery::discover_presets_scoped(Scope::Project).unwrap();
    assert_eq!(paths.len(), 0);
}
