mod common;

use claudio_core::preset::resolve::{self, ResolverConfig};
use claudio_core::preset::store::PresetStore;
use claudio_core::preset::types::PresetSource;
use claudio_core::preset::{io, lookup};
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
fn test_variable_substitution_in_env() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Set environment variable for substitution
    unsafe {
        std::env::set_var("API_KEY", "secret-key");
    }

    // Create preset with variable substitution
    env.write_project_preset(
        "test-preset",
        r#"{
  "name": "test-preset",
  "env": {
    "AUTH_TOKEN": "${API_KEY}",
    "BASE_URL": "https://api.example.com"
  }
}"#,
    );

    // Load and resolve preset
    let path = lookup::find_preset_scoped("test-preset", Scope::Project).unwrap();
    let preset = io::load_preset(&path).unwrap();

    let resolved = resolve::resolve_variables_with_source(
        &preset,
        &PresetSource::File(path),
        &ResolverConfig::default(),
    )
    .unwrap();

    // Check resolved values
    assert_eq!(resolved.get("AUTH_TOKEN"), Some(&"secret-key".to_string()));
    assert_eq!(
        resolved.get("BASE_URL"),
        Some(&"https://api.example.com".to_string())
    );

    // Cleanup
    unsafe {
        std::env::remove_var("API_KEY");
    }
    env.cleanup_env_vars();
}

#[test]
#[serial_test::serial]
fn test_missing_environment_variable() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Ensure env var is not set
    unsafe {
        std::env::remove_var("MISSING_VAR");
    }

    // Create preset with missing variable
    env.write_project_preset(
        "test-preset",
        r#"{
  "name": "test-preset",
  "env": {
    "AUTH_TOKEN": "${MISSING_VAR}"
  }
}"#,
    );

    // Load and attempt to resolve preset
    let path = lookup::find_preset_scoped("test-preset", Scope::Project).unwrap();
    let preset = io::load_preset(&path).unwrap();

    let result = resolve::resolve_variables_with_source(
        &preset,
        &PresetSource::File(path),
        &ResolverConfig::default(),
    );

    // Should return error
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(error.contains("MISSING_VAR"));

    env.cleanup_env_vars();
}

#[test]
#[serial_test::serial]
fn test_inheritance_resolution() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create base preset
    env.write_project_preset(
        "base",
        r#"{
  "name": "base",
  "env": {
    "VAR1": "base_value",
    "VAR2": "base_value2"
  },
  "args": ["--base-arg"]
}"#,
    );

    // Create derived preset
    env.write_project_preset(
        "derived",
        r#"{
  "name": "derived",
  "extends": "base",
  "env": {
    "VAR2": "derived_value2",
    "VAR3": "derived_value3"
  },
  "args": ["--derived-arg"]
}"#,
    );

    let store = PresetStore::new(Scope::Project).unwrap();
    let located = store.find_required("derived").unwrap();

    // Resolve inheritance
    let resolved = resolve::resolve_inheritance_with_store(
        &located.preset,
        located.source,
        &store,
        &ResolverConfig::default(),
    )
    .unwrap();

    // Check merged env vars (derived overrides base)
    assert_eq!(resolved.env.get("VAR1"), Some(&"base_value".to_string()));
    assert_eq!(
        resolved.env.get("VAR2"),
        Some(&"derived_value2".to_string())
    );
    assert_eq!(
        resolved.env.get("VAR3"),
        Some(&"derived_value3".to_string())
    );

    // Check args are appended
    assert_eq!(resolved.args, vec!["--base-arg", "--derived-arg"]);

    env.cleanup_env_vars();
}

#[test]
#[serial_test::serial]
fn test_circular_inheritance_detection() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create preset A extending B
    env.write_project_preset(
        "preset-a",
        r#"{
  "name": "preset-a",
  "extends": "preset-b",
  "env": {}
}"#,
    );

    // Create preset B extending A
    env.write_project_preset(
        "preset-b",
        r#"{
  "name": "preset-b",
  "extends": "preset-a",
  "env": {}
}"#,
    );

    let store = PresetStore::new(Scope::Project).unwrap();
    let located = store.find_required("preset-a").unwrap();
    let result = resolve::resolve_inheritance_with_store(
        &located.preset,
        located.source,
        &store,
        &ResolverConfig::default(),
    );

    // Should return error for circular dependency
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(error.contains("Circular"));
}

#[test]
#[serial_test::serial]
fn test_self_referencing_extends() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create preset extending itself
    env.write_project_preset(
        "self-ref",
        r#"{
  "name": "self-ref",
  "extends": "self-ref",
  "env": {}
}"#,
    );

    let store = PresetStore::new(Scope::Project).unwrap();
    let located = store.find_required("self-ref").unwrap();
    let result = resolve::resolve_inheritance_with_store(
        &located.preset,
        located.source,
        &store,
        &ResolverConfig::default(),
    );

    // Should return error
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("Circular inheritance") || error.contains("Circular"),
        "Unexpected error message: {}",
        error
    );
}

#[test]
#[serial_test::serial]
fn test_complex_variable_substitution() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Set multiple environment variables
    unsafe {
        std::env::set_var("HOST", "example.com");
        std::env::set_var("PORT", "8080");
        std::env::set_var("PATH", "/api/v1");
    }

    // Create preset with multiple substitutions
    env.write_project_preset(
        "test-preset",
        r#"{
  "name": "test-preset",
  "env": {
    "URL": "https://${HOST}:${PORT}${PATH}",
    "SIMPLE": "${HOST}"
  }
}"#,
    );

    // Load and resolve
    let path = lookup::find_preset_scoped("test-preset", Scope::Project).unwrap();
    let preset = io::load_preset(&path).unwrap();

    let resolved = resolve::resolve_variables_with_source(
        &preset,
        &PresetSource::File(path),
        &ResolverConfig::default(),
    )
    .unwrap();

    // Check resolved values
    assert_eq!(
        resolved.get("URL"),
        Some(&"https://example.com:8080/api/v1".to_string())
    );
    assert_eq!(resolved.get("SIMPLE"), Some(&"example.com".to_string()));

    // Cleanup
    unsafe {
        std::env::remove_var("HOST");
        std::env::remove_var("PORT");
        std::env::remove_var("PATH");
    }
    env.cleanup_env_vars();
}

#[test]
#[serial_test::serial]
fn test_resolved_preset_includes_settings() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create preset with settings
    env.write_project_preset(
        "test-preset",
        r#"{
  "name": "test-preset",
  "settings": {
    "model": "sonnet",
    "permissionMode": "acceptEdits"
  }
}"#,
    );

    // Load preset
    let path = lookup::find_preset_scoped("test-preset", Scope::Project).unwrap();
    let preset = io::load_preset(&path).unwrap();

    // Settings should be preserved
    assert!(preset.settings.is_some());
    let settings = preset.settings.as_ref().unwrap();
    assert_eq!(
        settings.get("model"),
        Some(&serde_json::Value::String("sonnet".to_string()))
    );
    assert_eq!(
        settings.get("permissionMode"),
        Some(&serde_json::Value::String("acceptEdits".to_string()))
    );
}

#[test]
#[serial_test::serial]
fn test_missing_required_field() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.setup_env_vars();
    let _cwd = CwdGuard::new(&env.project_dir);

    // Create preset missing required name
    env.write_project_preset(
        "invalid",
        r#"{
  "description": "Missing name",
  "env": {}
}"#,
    );

    // Loading should return error
    let path = env
        .project_dir
        .join(".claudio")
        .join("presets")
        .join("invalid.json");
    let result = io::load_preset(&path);
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(error.contains("name"));
}
