//! End-to-end CLI integration tests.

mod common;

use common::TestEnvironment;
use serial_test::serial;
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
    run_claudio_with_cwd(args, env_vars, None)
}

/// Run the claudio binary with the given arguments and working directory.
fn run_claudio_with_cwd(
    args: &[&str],
    env_vars: &[(&str, &str)],
    cwd: Option<&std::path::Path>,
) -> std::process::Output {
    let binary = get_binary_path();

    let mut cmd = Command::new(&binary);
    cmd.args(args);

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
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
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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
#[serial]
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

// ─────────────────────────────────────────────────────────────────────────────
// Run Command Tests - Shorthand Syntax
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn test_cli_run_shorthand_syntax_nonexistent_preset() {
    let env = TestEnvironment::new();

    let output = run_claudio(
        &["nonexistent-preset", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("nonexistent-preset"),
        "Error message should indicate preset was not found"
    );
}

#[test]
#[serial]
fn test_cli_run_shorthand_with_scope_flag() {
    let env = TestEnvironment::new();

    let preset_json = r#"{"name": "test-preset"}"#;
    env.create_preset("user", "test-preset", preset_json);

    let output = run_claudio(
        &["test-preset", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    // Will fail with "Failed to execute 'claude'" but should find the preset first
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("using preset test-preset"),
        "Should indicate it's using the preset"
    );
}

#[test]
#[serial]
fn test_cli_run_shorthand_with_require_preset_flag() {
    let env = TestEnvironment::new();

    let output = run_claudio(
        &["--require-preset", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("require_preset") || stderr.contains("No preset"),
        "Should error about missing preset when require_preset is enabled"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Run Command Tests - Explicit vs Shorthand Equivalence
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn test_cli_run_explicit_vs_shorthand_equivalence() {
    let env = TestEnvironment::new();

    let preset_json = r#"{"name": "equivalence-test"}"#;
    env.create_preset("user", "equivalence-test", preset_json);

    let output_explicit = run_claudio(
        &["run", "equivalence-test", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    let output_shorthand = run_claudio(
        &["equivalence-test", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    let stderr_explicit = String::from_utf8_lossy(&output_explicit.stderr);
    let stderr_shorthand = String::from_utf8_lossy(&output_shorthand.stderr);

    // Both should indicate they're using the same preset
    assert!(
        stderr_explicit.contains("using preset equivalence-test"),
        "Explicit run should use the preset"
    );
    assert!(
        stderr_shorthand.contains("using preset equivalence-test"),
        "Shorthand should use the preset"
    );

    // Both should have the same exit status (both will fail to execute claude)
    assert_eq!(
        output_explicit.status.code(),
        output_shorthand.status.code(),
        "Both syntaxes should produce the same exit code"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Run Command Tests - Reserved Name Precedence
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn test_cli_reserved_names_invoke_subcommands() {
    let output_init = run_claudio(&["init", "--help"], &[]);
    assert!(output_init.status.success());
    let stdout_init = String::from_utf8_lossy(&output_init.stdout);
    assert!(
        stdout_init.contains("Initialize"),
        "init should show init subcommand help"
    );

    let output_preset = run_claudio(&["preset", "--help"], &[]);
    assert!(output_preset.status.success());
    let stdout_preset = String::from_utf8_lossy(&output_preset.stdout);
    assert!(
        stdout_preset.contains("Manage presets"),
        "preset should show preset subcommand help"
    );

    let output_run = run_claudio(&["run", "--help"], &[]);
    assert!(output_run.status.success());
    let stdout_run = String::from_utf8_lossy(&output_run.stdout);
    assert!(
        stdout_run.contains("Run Claude Code"),
        "run should show run subcommand help"
    );
}

#[test]
#[serial]
fn test_cli_reserved_name_preset_requires_explicit_run() {
    let env = TestEnvironment::new();

    // Create a preset named "init"
    let preset_json = r#"{"name": "init", "description": "A preset with reserved name"}"#;
    env.create_preset("user", "init", preset_json);

    // Using shorthand syntax should invoke the init subcommand, not the preset
    let output_shorthand = run_claudio(
        &["init", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    // Should succeed (creates directories)
    assert!(
        output_shorthand.status.success(),
        "Shorthand 'init' should invoke init subcommand"
    );

    // Using explicit run should use the preset
    let output_explicit = run_claudio(
        &["run", "init", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    let stderr_explicit = String::from_utf8_lossy(&output_explicit.stderr);
    assert!(
        stderr_explicit.contains("using preset init"),
        "Explicit 'run init' should use the init preset"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Run Command Tests - Global Flags with Shorthand
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn test_cli_run_shorthand_with_color_flag() {
    let env = TestEnvironment::new();

    let preset_json = r#"{"name": "color-flag-test"}"#;
    env.create_preset("user", "color-flag-test", preset_json);

    let output = run_claudio(
        &["--color", "green", "color-flag-test", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    // Will fail to execute claude, but should process the preset
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("using preset color-flag-test"),
        "Should process preset with --color flag"
    );
}

#[test]
#[serial]
fn test_cli_run_shorthand_with_no_color_flag() {
    let env = TestEnvironment::new();

    let preset_json = r#"{"name": "no-color-test"}"#;
    env.create_preset("user", "no-color-test", preset_json);

    let output = run_claudio(
        &["--no-color", "no-color-test", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should process preset
    assert!(
        stderr.contains("using preset no-color-test"),
        "Should process preset with --no-color flag"
    );
    // Stderr should not contain ANSI codes
    assert!(
        !stderr.contains("\x1b["),
        "Should not contain ANSI codes with --no-color"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Run Command Tests - Default Preset Behavior
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn test_cli_run_shorthand_without_preset_name_uses_default() {
    let env = TestEnvironment::new();

    // Create a preset named "default"
    let preset_json = r#"{"name": "default", "description": "Default preset"}"#;
    env.create_preset("user", "default", preset_json);

    let output = run_claudio(
        &["--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("using preset default") || stderr.contains("using preset \"default\""),
        "Should use default preset when no preset name is provided"
    );
}

#[test]
#[serial]
fn test_cli_run_explicit_without_preset_name_uses_default() {
    let env = TestEnvironment::new();

    // Create a preset named "default"
    let preset_json = r#"{"name": "default", "description": "Default preset"}"#;
    env.create_preset("user", "default", preset_json);

    let output = run_claudio(
        &["run", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("using preset default") || stderr.contains("using preset \"default\""),
        "Explicit run should also use default preset when no name is provided"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Run Command Tests - Edge Cases
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[serial]
fn test_cli_run_shorthand_with_scope_precedence() {
    let env = TestEnvironment::new();

    // Create preset in user scope
    let user_preset = r#"{"name": "shadow-test", "env": {"SCOPE": "user"}}"#;
    env.create_preset("user", "shadow-test", user_preset);

    // Create preset in project scope with different env var
    let project_preset = r#"{"name": "shadow-test", "env": {"SCOPE": "project"}}"#;
    env.create_preset("project", "shadow-test", project_preset);

    // Test that project scope works when running from project directory
    let project_output = run_claudio_with_cwd(
        &["preset", "env", "shadow-test", "--scope", "project"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
        Some(&env.project_path()),
    );

    let project_stdout = String::from_utf8_lossy(&project_output.stdout);
    assert!(
        project_stdout.contains("SCOPE") && project_stdout.contains("project"),
        "Should use project-scoped preset when scope=project"
    );

    // Test that user scope can be explicitly selected
    let user_output = run_claudio(
        &["preset", "env", "shadow-test", "--scope", "user"],
        &[("CLAUDIO_HOME_DIR", env.home_path().to_str().unwrap())],
    );

    let user_stdout = String::from_utf8_lossy(&user_output.stdout);
    assert!(
        user_stdout.contains("SCOPE") && user_stdout.contains("user"),
        "Should use user-scoped preset when scope=user"
    );
}
