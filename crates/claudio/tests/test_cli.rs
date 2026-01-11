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

#[test]
#[serial]
fn test_preset_add_with_extends_flag() {
    let env = TestEnvironment::new();
    env.create_project_config();

    // Create a base preset first
    env.write_project_preset(
        "base",
        r#"{
  "name": "base",
  "description": "Base preset",
  "env": {
    "BASE_VAR": "base_value"
  }
}"#,
    );

    // Run preset add command with --extends flag
    let output = run_claudio(
        &env.project_dir,
        &[
            "preset",
            "add",
            "derived",
            "--extends",
            "base",
            "--scope",
            "project",
        ],
        &[
            ("CLAUDIO_HOME_DIR", env.home_dir.to_str().unwrap()),
            ("EDITOR", "/usr/bin/true"), // Avoid opening an editor
        ],
    );

    assert!(
        output.status.success(),
        "Preset add with --extends failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify preset file was created with extends field
    let preset_path = env
        .project_dir
        .join(".claudio")
        .join("presets")
        .join("derived.json");
    assert!(preset_path.exists());

    // Verify preset file contains extends field
    let content = std::fs::read_to_string(&preset_path).unwrap();
    assert!(content.contains("\"name\": \"derived\""));
    assert!(content.contains("\"extends\": \"base\""));
}

#[test]
#[serial]
fn test_preset_add_with_extend_alias() {
    let env = TestEnvironment::new();
    env.create_project_config();

    // Create a base preset first
    env.write_project_preset(
        "base",
        r#"{
  "name": "base",
  "description": "Base preset"
}"#,
    );

    // Run preset add command with --extend alias
    let output = run_claudio(
        &env.project_dir,
        &[
            "preset", "add", "derived", "--extend", "base", "--scope", "project",
        ],
        &[
            ("CLAUDIO_HOME_DIR", env.home_dir.to_str().unwrap()),
            ("EDITOR", "/usr/bin/true"),
        ],
    );

    assert!(
        output.status.success(),
        "Preset add with --extend alias failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify preset file contains extends field
    let preset_path = env
        .project_dir
        .join(".claudio")
        .join("presets")
        .join("derived.json");
    let content = std::fs::read_to_string(&preset_path).unwrap();
    assert!(content.contains("\"extends\": \"base\""));
}

#[test]
#[serial]
fn test_preset_add_without_extends_has_null_extends() {
    let env = TestEnvironment::new();
    env.create_project_config();

    // Run preset add command without --extends flag
    let output = run_claudio(
        &env.project_dir,
        &["preset", "add", "standalone", "--scope", "project"],
        &[
            ("CLAUDIO_HOME_DIR", env.home_dir.to_str().unwrap()),
            ("EDITOR", "/usr/bin/true"),
        ],
    );

    assert!(
        output.status.success(),
        "Preset add without --extends failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify preset file has null extends field
    let preset_path = env
        .project_dir
        .join(".claudio")
        .join("presets")
        .join("standalone.json");
    let content = std::fs::read_to_string(&preset_path).unwrap();
    assert!(
        content.contains("\"extends\": null"),
        "Expected extends to be null, got: {}",
        content
    );
}

#[test]
#[serial]
fn test_preset_add_with_nonexistent_base_fails() {
    let env = TestEnvironment::new();
    env.create_project_config();

    // Run preset add command with non-existent base preset
    let output = run_claudio(
        &env.project_dir,
        &[
            "preset",
            "add",
            "derived",
            "--extends",
            "nonexistent",
            "--scope",
            "project",
        ],
        &[
            ("CLAUDIO_HOME_DIR", env.home_dir.to_str().unwrap()),
            ("EDITOR", "/usr/bin/true"),
        ],
    );

    assert!(
        !output.status.success(),
        "Preset add with non-existent base should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("nonexistent") && stderr.contains("not found"),
        "Expected error about non-existent preset, got: {}",
        stderr
    );

    // Verify preset file was NOT created
    let preset_path = env
        .project_dir
        .join(".claudio")
        .join("presets")
        .join("derived.json");
    assert!(!preset_path.exists());
}

#[test]
#[serial]
fn test_preset_add_extends_finds_base_in_user_scope() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.create_user_config();

    // Create base preset in user scope
    env.write_user_preset(
        "user-base",
        r#"{
  "name": "user-base",
  "description": "User-level base preset"
}"#,
    );

    // Run preset add command extending user preset from project scope
    let output = run_claudio(
        &env.project_dir,
        &[
            "preset",
            "add",
            "project-derived",
            "--extends",
            "user-base",
            "--scope",
            "project",
        ],
        &[
            ("CLAUDIO_HOME_DIR", env.home_dir.to_str().unwrap()),
            ("EDITOR", "/usr/bin/true"),
        ],
    );

    assert!(
        output.status.success(),
        "Should be able to extend preset from user scope\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify preset file contains extends field
    let preset_path = env
        .project_dir
        .join(".claudio")
        .join("presets")
        .join("project-derived.json");
    let content = std::fs::read_to_string(&preset_path).unwrap();
    assert!(content.contains("\"extends\": \"user-base\""));
}

#[test]
#[serial]
fn test_doctor_command_basic() {
    let env = TestEnvironment::new();
    env.create_project_config();

    // Run doctor command
    let output = run_claudio(
        &env.project_dir,
        &["doctor"],
        &[
            ("CLAUDIO_HOME_DIR", env.home_dir.to_str().unwrap()),
            ("CLAUDIO_CLAUDE_EXECUTABLE", "true"),
        ],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Doctor should succeed (exit code 0 or 1 for warnings)
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "Doctor command failed\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify output contains expected checks
    assert!(stdout.contains("[OK]") || stdout.contains("[WARN]") || stdout.contains("[FAIL]"));
    assert!(stdout.contains("Summary:"));
    assert!(stdout.contains("Exit code:"));
}

#[test]
#[serial]
fn test_doctor_command_json_output() {
    let env = TestEnvironment::new();
    env.create_project_config();

    // Run doctor command with --json flag
    let output = run_claudio(
        &env.project_dir,
        &["doctor", "--json"],
        &[
            ("CLAUDIO_HOME_DIR", env.home_dir.to_str().unwrap()),
            ("CLAUDIO_CLAUDE_EXECUTABLE", "true"),
        ],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should succeed
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "Doctor --json command failed\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify JSON output is valid
    let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(
        json.is_ok(),
        "Doctor JSON output is not valid JSON:\n{}",
        stdout
    );

    let json_obj = json.unwrap();
    assert!(json_obj.get("checks").is_some());
    assert!(json_obj.get("summary").is_some());
    assert!(json_obj.get("exit_code").is_some());
}

#[test]
#[serial]
fn test_doctor_command_fails_when_executable_missing() {
    let env = TestEnvironment::new();
    env.create_project_config();

    // Run doctor command with a bogus executable
    let output = run_claudio(
        &env.project_dir,
        &["doctor", "--json"],
        &[
            ("CLAUDIO_HOME_DIR", env.home_dir.to_str().unwrap()),
            (
                "CLAUDIO_CLAUDE_EXECUTABLE",
                "definitely-not-a-real-claude-executable-12345",
            ),
        ],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(
        output.status.code(),
        Some(2),
        "Doctor should fail when executable is missing\nstdout:\n{}\nstderr:\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );

    let json_obj: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| panic!("Doctor JSON output is not valid JSON:\n{}", stdout));

    assert_eq!(
        json_obj.get("exit_code").and_then(|v| v.as_i64()),
        Some(2),
        "Expected exit_code=2 in doctor JSON\n{}",
        stdout
    );

    let checks = json_obj
        .get("checks")
        .and_then(|v| v.as_array())
        .expect("Expected checks array in doctor JSON");

    let claude_check = checks
        .iter()
        .find(|c| c.get("name").and_then(|v| v.as_str()) == Some("claude_executable"))
        .expect("Expected a claude_executable check in doctor JSON");

    assert_eq!(
        claude_check.get("status").and_then(|v| v.as_str()),
        Some("fail"),
        "Expected claude_executable status=fail\n{}",
        stdout
    );
}
