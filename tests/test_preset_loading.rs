//! Integration tests for preset loading and discovery.

mod common;

use claudio::preset::loader;
use common::TestEnvironment;
use std::fs;

// ─────────────────────────────────────────────────────────────────────────────
// Preset Loading Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_load_valid_preset() {
    let env = TestEnvironment::new();

    let preset_json = r#"{
        "name": "test-preset",
        "description": "A test preset",
        "env": {
            "API_URL": "https://example.com"
        },
        "args": ["--verbose"]
    }"#;

    let path = env.create_preset("user", "test-preset", preset_json);

    let preset = loader::load_preset(&path).expect("Failed to load preset");
    assert_eq!(preset.name, "test-preset");
    assert_eq!(preset.description, Some("A test preset".to_string()));
    assert!(preset.env.is_some());
    assert_eq!(preset.args, Some(vec!["--verbose".to_string()]));
}

#[test]
fn test_load_minimal_preset() {
    let env = TestEnvironment::new();

    let preset_json = r#"{"name": "minimal"}"#;

    let path = env.create_preset("user", "minimal", preset_json);

    let preset = loader::load_preset(&path).expect("Failed to load preset");
    assert_eq!(preset.name, "minimal");
    assert!(preset.description.is_none());
    assert!(preset.env.is_none());
    assert!(preset.args.is_none());
}

#[test]
fn test_load_preset_with_settings() {
    let env = TestEnvironment::new();

    let preset_json = r#"{
        "name": "with-settings",
        "settings": {
            "model": "sonnet",
            "permission-mode": "acceptEdits"
        }
    }"#;

    let path = env.create_preset("user", "with-settings", preset_json);

    let preset = loader::load_preset(&path).expect("Failed to load preset");
    assert!(preset.settings.is_some());
    let settings = preset.settings.unwrap();
    assert_eq!(settings["model"], "sonnet");
}

#[test]
fn test_load_invalid_json() {
    let env = TestEnvironment::new();

    let invalid_json = r#"{"name": "broken", invalid}"#;
    let path = env.create_preset("user", "broken", invalid_json);

    let result = loader::load_preset(&path);
    assert!(result.is_err());
}

#[test]
fn test_load_nonexistent_file() {
    let path = std::path::Path::new("/nonexistent/path/preset.json");
    let result = loader::load_preset(path);
    assert!(result.is_err());
}

// ─────────────────────────────────────────────────────────────────────────────
// Default Preset Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_default_preset_creation() {
    let preset = loader::default_preset("my-preset");

    assert_eq!(preset.name, "my-preset");
    assert!(preset.description.is_some());
    assert!(preset.env.is_some());
    assert!(preset.args.is_some());
}

#[test]
fn test_preset_path_for_name() {
    let dir = std::path::Path::new("/some/dir");
    let path = loader::preset_path_for_name(dir, "my-preset");

    assert_eq!(path, std::path::PathBuf::from("/some/dir/my-preset.json"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Preset Validation Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_preset_validation_on_load() {
    let env = TestEnvironment::new();

    // Create a preset with valid structure but invalid env key
    let preset_json = r#"{
        "name": "valid-structure",
        "env": {
            "VALID_KEY": "value"
        }
    }"#;

    let path = env.create_preset("user", "valid-structure", preset_json);
    let preset = loader::load_preset(&path).expect("Failed to load preset");

    let validation = preset.validate(Some(&path));
    assert!(validation.is_valid());
}

// ─────────────────────────────────────────────────────────────────────────────
// Find Project Root Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_find_project_root_with_git() {
    let temp_dir = tempfile::tempdir().unwrap();
    let project_dir = temp_dir.path().join("my-project");
    fs::create_dir_all(&project_dir).unwrap();

    // Create .git directory
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    // Create a nested directory
    let nested = project_dir.join("src/nested");
    fs::create_dir_all(&nested).unwrap();

    let found = loader::find_project_root(&nested);
    assert!(found.is_some());
    assert_eq!(found.unwrap(), project_dir);
}

#[test]
fn test_find_project_root_with_claudio_dir() {
    let temp_dir = tempfile::tempdir().unwrap();
    let project_dir = temp_dir.path().join("rust-project");
    fs::create_dir_all(&project_dir).unwrap();

    // Create .claudio directory (project marker)
    fs::create_dir_all(project_dir.join(".claudio")).unwrap();

    let nested = project_dir.join("src");
    fs::create_dir_all(&nested).unwrap();

    let found = loader::find_project_root(&nested);
    assert!(found.is_some());
    assert_eq!(found.unwrap(), project_dir);
}

#[test]
fn test_find_project_root_prefers_closest() {
    let temp_dir = tempfile::tempdir().unwrap();
    let outer_dir = temp_dir.path().join("outer");
    let inner_dir = outer_dir.join("inner");
    fs::create_dir_all(&inner_dir).unwrap();

    // Create .git in outer
    fs::create_dir_all(outer_dir.join(".git")).unwrap();
    // Create .claudio in inner
    fs::create_dir_all(inner_dir.join(".claudio")).unwrap();

    let work_dir = inner_dir.join("src");
    fs::create_dir_all(&work_dir).unwrap();

    // Should find inner (closest marker)
    let found = loader::find_project_root(&work_dir);
    assert!(found.is_some());
    assert_eq!(found.unwrap(), inner_dir);
}

#[test]
fn test_find_project_root_git_also_works() {
    let temp_dir = tempfile::tempdir().unwrap();
    let project_dir = temp_dir.path().join("git-project");
    fs::create_dir_all(&project_dir).unwrap();

    // Create only .git directory (no .claudio)
    fs::create_dir_all(project_dir.join(".git")).unwrap();

    let nested = project_dir.join("src");
    fs::create_dir_all(&nested).unwrap();

    let found = loader::find_project_root(&nested);
    assert!(found.is_some());
    assert_eq!(found.unwrap(), project_dir);
}

#[test]
fn test_find_project_root_none() {
    let temp_dir = tempfile::tempdir().unwrap();
    let empty_dir = temp_dir.path().join("deeply/nested/empty/dir");
    fs::create_dir_all(&empty_dir).unwrap();

    // No markers exist within temp_dir, so any found root must be above temp_dir
    let found = loader::find_project_root(&empty_dir);

    match found {
        None => {
            // Expected in isolated environments - no markers found anywhere
        }
        Some(root) => {
            // If a root is found, it must be ABOVE our temp directory
            // (e.g., a .git in /tmp or user's home), not within it
            assert!(
                !root.starts_with(temp_dir.path()),
                "Found root {:?} should not be within temp dir {:?}",
                root,
                temp_dir.path()
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Atomic Write Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_write_preset_atomic() {
    let env = TestEnvironment::new();
    let preset = loader::default_preset("atomic-test");
    let path = env.user_preset_dir.join("atomic-test.json");

    loader::write_preset_atomic(&path, &preset).expect("Failed to write preset");

    // Verify the file was created
    assert!(path.exists());

    // Verify we can load it back
    let loaded = loader::load_preset(&path).expect("Failed to load written preset");
    assert_eq!(loaded.name, "atomic-test");
}
