mod common;

use common::TestEnvironment;
use serial_test::serial;
use std::path::Path;
use std::process::Command;

fn run_claudio(cwd: &Path, args: &[&str], env_vars: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_claudio"));
    cmd.args(args);
    cmd.current_dir(cwd);

    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    cmd.output().expect("Failed to execute command")
}

#[test]
#[serial]
fn test_init_command_creates_directories() {
    let env = TestEnvironment::new();

    // Ensure project root detection works for `--scope project`
    std::fs::create_dir_all(env.project_dir.join(".git")).unwrap();

    // Run init command
    let output = run_claudio(
        &env.project_dir,
        &["init", "--scope", "project"],
        &[("CLAUDIO_HOME_DIR", env.home_dir.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "Init command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify directories were created
    assert!(env.project_dir.join(".claudio").exists());
    assert!(env.project_dir.join(".claudio").join("presets").exists());
    assert!(
        env.project_dir
            .join(".claudio")
            .join("settings.json")
            .exists()
    );
}

#[test]
#[serial]
fn test_preset_add_command_creates_preset_file() {
    let env = TestEnvironment::new();
    env.create_project_config();

    // Run preset add command
    let output = run_claudio(
        &env.project_dir,
        &["preset", "add", "test-preset", "--scope", "project"],
        &[
            ("CLAUDIO_HOME_DIR", env.home_dir.to_str().unwrap()),
            ("EDITOR", "/usr/bin/true"), // Avoid opening an editor
        ],
    );

    assert!(
        output.status.success(),
        "Preset add command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify preset file was created
    let preset_path = env
        .project_dir
        .join(".claudio")
        .join("presets")
        .join("test-preset.json");
    assert!(preset_path.exists());

    // Verify preset file contains expected template
    let content = std::fs::read_to_string(&preset_path).unwrap();
    assert!(content.contains("\"name\": \"test-preset\""));
    assert!(content.contains("\"description\":"));
    assert!(content.contains("\"env\":"));
    assert!(content.contains("\"args\":"));
}

#[test]
#[serial]
fn test_preset_list_command_shows_available_presets() {
    let env = TestEnvironment::new();
    env.create_project_config();

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

    // Run preset list command
    let output = run_claudio(
        &env.project_dir,
        &["preset", "list"],
        &[("CLAUDIO_HOME_DIR", env.home_dir.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "Preset list command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test-preset"));
    assert!(stdout.contains("Test preset"));
}

#[test]
#[serial]
fn test_preset_show_command_displays_preset_details() {
    let env = TestEnvironment::new();
    env.create_project_config();

    // Create test preset
    env.write_project_preset(
        "test-preset",
        r#"{
  "name": "test-preset",
  "description": "Test preset",
  "env": {
    "TEST_VAR": "test_value"
  },
  "args": ["--test"]
}"#,
    );

    // Run preset show command
    let output = run_claudio(
        &env.project_dir,
        &["preset", "show", "test-preset"],
        &[("CLAUDIO_HOME_DIR", env.home_dir.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "Preset show command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test-preset"));
    assert!(stdout.contains("Test preset"));
    assert!(stdout.contains("TEST_VAR"));
    assert!(stdout.contains("test_value"));
    assert!(stdout.contains("--test"));
}

#[test]
#[serial]
fn test_invalid_command_returns_error() {
    let env = TestEnvironment::new();

    // Ensure project root detection works (otherwise CLI may fail before parsing)
    std::fs::create_dir_all(env.project_dir.join(".git")).unwrap();

    // Run invalid command
    let output = run_claudio(
        &env.project_dir,
        &["invalid-command"],
        &[("CLAUDIO_HOME_DIR", env.home_dir.to_str().unwrap())],
    );

    assert!(!output.status.success(), "Invalid command should fail");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Preset 'invalid-command' not found")
            || stderr.contains("Failed to find preset")
            || stderr.contains("not found"),
        "Expected error message, got: {}",
        stderr
    );
}
