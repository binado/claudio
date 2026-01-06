//! End-to-end CLI integration tests.

mod common;

use common::TestEnvironment;
use std::path::PathBuf;
use std::process::Command;

/// Get the path to the built binary.
fn get_binary_path() -> PathBuf {
    // The binary should be in target/debug/claudio during testing
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("claudio");
    path
}

/// Run the claudio binary with the given arguments.
fn run_claudio(args: &[&str], env_vars: &[(&str, &str)]) -> std::process::Output {
    let binary = get_binary_path();

    let mut cmd = Command::new(&binary);
    cmd.args(args);

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    cmd.output().expect("Failed to execute claudio binary")
}

// ─────────────────────────────────────────────────────────────────────────────
// Help and Version Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_cli_help() {
    let output = run_claudio(&["--help"], &[]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("claudio"));
}

#[test]
fn test_cli_version() {
    let output = run_claudio(&["--version"], &[]);
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("claudio"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Init Command Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_cli_init_creates_directories() {
    let env = TestEnvironment::new();

    // Remove the preset dirs that TestEnvironment creates
    std::fs::remove_dir_all(&env.user_preset_dir).ok();

    let output = run_claudio(
        &["init", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    // Should succeed
    assert!(
        output.status.success(),
        "init should succeed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Should have created the preset directory
    assert!(
        env.user_preset_dir.exists(),
        "preset directory should be created"
    );

    // Should have created settings.json
    assert!(
        env.user_settings_path.exists(),
        "settings.json should be created"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// List Command Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_cli_list_empty() {
    let env = TestEnvironment::new();

    // No presets created, directory is empty
    let output = run_claudio(
        &["preset", "list", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    // Should succeed even with no presets
    assert!(output.status.success());
}

#[test]
fn test_cli_list_with_presets() {
    let env = TestEnvironment::new();

    // Create a valid preset
    let preset_json = r#"{
        "name": "test-preset",
        "description": "A test preset for listing"
    }"#;
    env.create_preset("user", "test-preset", preset_json);

    let output = run_claudio(
        &["preset", "list", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test-preset"));
}

#[test]
fn test_cli_list_verbose() {
    let env = TestEnvironment::new();

    let preset_json = r#"{
        "name": "verbose-test",
        "description": "Testing verbose output",
        "extends": "none",
        "env": { "TEST_VAR": "value" },
        "args": ["--test"]
    }"#;
    env.create_preset("user", "verbose-test", preset_json);

    let output = run_claudio(
        &["preset", "list", "--scope", "user", "--verbose"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("verbose-test"));
}

#[test]
fn test_cli_list_with_fields() {
    let env = TestEnvironment::new();

    let preset_json = r#"{
        "name": "fields-test",
        "description": "Testing field selection"
    }"#;
    env.create_preset("user", "fields-test", preset_json);

    let output = run_claudio(
        &[
            "preset",
            "list",
            "--scope",
            "user",
            "--fields",
            "name,description",
        ],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("fields-test"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Show Command Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_cli_show_preset() {
    let env = TestEnvironment::new();

    let preset_json = r#"{
        "name": "show-test",
        "description": "Testing show command",
        "env": { "API_KEY": "secret" }
    }"#;
    env.create_preset("user", "show-test", preset_json);

    let output = run_claudio(
        &["preset", "show", "show-test", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("show-test"));
    assert!(stdout.contains("Testing show command"));
}

#[test]
fn test_cli_show_json_output() {
    let env = TestEnvironment::new();

    let preset_json = r#"{
        "name": "json-test",
        "description": "Testing JSON output"
    }"#;
    env.create_preset("user", "json-test", preset_json);

    let output = run_claudio(
        &["preset", "show", "json-test", "--scope", "user", "--json"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should be valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON");
}

#[test]
fn test_cli_show_nonexistent() {
    let env = TestEnvironment::new();

    let output = run_claudio(
        &["preset", "show", "nonexistent", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("nonexistent") || stderr.contains("not found"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Add Command Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_cli_add_existing_preset_fails() {
    let env = TestEnvironment::new();

    // Create an existing preset
    let preset_json = r#"{"name": "existing"}"#;
    env.create_preset("user", "existing", preset_json);

    // Attempting to add should fail (without --force or equivalent)
    let output = run_claudio(
        &["preset", "add", "existing", "--scope", "user"],
        &[
            ("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap()),
            ("EDITOR", "true"), // Use 'true' as a no-op editor
        ],
    );

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Env Command Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_cli_env_command() {
    let env = TestEnvironment::new();

    let preset_json = r#"{
        "name": "env-test",
        "env": {
            "API_URL": "https://api.example.com",
            "API_KEY": "test-key"
        }
    }"#;
    env.create_preset("user", "env-test", preset_json);

    let output = run_claudio(
        &["preset", "env", "env-test", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("API_URL"));
    assert!(stdout.contains("API_KEY"));
}

#[test]
fn test_cli_env_export_format() {
    let env = TestEnvironment::new();

    let preset_json = r#"{
        "name": "export-test",
        "env": {
            "MY_VAR": "my-value"
        }
    }"#;
    env.create_preset("user", "export-test", preset_json);

    let output = run_claudio(
        &[
            "preset",
            "env",
            "export-test",
            "--scope",
            "user",
            "--export",
        ],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("export MY_VAR="));
}

// ─────────────────────────────────────────────────────────────────────────────
// Color Flag Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_cli_no_color_flag() {
    let env = TestEnvironment::new();

    let preset_json = r#"{"name": "color-test"}"#;
    env.create_preset("user", "color-test", preset_json);

    let output = run_claudio(
        &["--no-color", "preset", "list", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    assert!(output.status.success());

    // Output should not contain ANSI escape codes when --no-color is used
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Check that it doesn't contain common ANSI escape sequences
    assert!(
        !stdout.contains("\x1b["),
        "Should not contain ANSI codes with --no-color"
    );
}
