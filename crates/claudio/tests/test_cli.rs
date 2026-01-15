mod common;

use claudio_core::env::CLAUDIO_HOME_DIR_ENV_VAR;
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

fn list_backup_files(dir: &Path) -> Vec<std::path::PathBuf> {
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("settings.json.") && name.ends_with(".backup") {
                Some(entry.path())
            } else {
                None
            }
        })
        .collect()
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
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
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
            (CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap()),
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
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
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
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
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
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
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
            (CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap()),
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
            (CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap()),
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
            (CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap()),
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
            (CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap()),
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
            (CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap()),
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
            (CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap()),
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
            (CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap()),
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
            (CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap()),
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

#[test]
#[serial]
fn test_init_dry_run_does_not_create_files() {
    let env = TestEnvironment::new();
    std::fs::create_dir_all(env.project_dir.join(".git")).unwrap();

    // Run init with --dry-run
    let output = run_claudio(
        &env.project_dir,
        &["init", "--scope", "project", "--dry-run"],
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "Init --dry-run failed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify no files were created
    assert!(!env.project_dir.join(".claudio").exists());

    // Verify dry-run message is shown
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("dry-run")
            || stderr.contains("would-backup")
            || stderr.contains("exists")
            || stderr.contains("would add"),
        "Expected dry-run related message in stderr: {}",
        stderr
    );
}

#[test]
#[serial]
fn test_init_reinitialize_shows_existing_items() {
    let env = TestEnvironment::new();
    std::fs::create_dir_all(env.project_dir.join(".git")).unwrap();

    // First init
    let output1 = run_claudio(
        &env.project_dir,
        &["init", "--scope", "project"],
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
    );
    assert!(output1.status.success());

    // Second init
    let output2 = run_claudio(
        &env.project_dir,
        &["init", "--scope", "project"],
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
    );
    assert!(output2.status.success());

    // Verify message indicates already initialized
    let stderr = String::from_utf8_lossy(&output2.stderr);
    assert!(
        stderr.contains("Already initialized") || stderr.contains("exists"),
        "Expected 'Already initialized' message: {}",
        stderr
    );
}

#[test]
#[serial]
fn test_init_quiet_suppresses_output() {
    let env = TestEnvironment::new();
    std::fs::create_dir_all(env.project_dir.join(".git")).unwrap();

    // Run init with --quiet
    let output = run_claudio(
        &env.project_dir,
        &["init", "--scope", "project", "--quiet"],
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "Init --quiet failed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify output is suppressed
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty() || !stderr.contains("Initializing"),
        "Expected quiet output, got: {}",
        stderr
    );

    // Verify files were still created
    assert!(env.project_dir.join(".claudio").exists());
    assert!(env.project_dir.join(".claudio/presets").exists());
    assert!(env.project_dir.join(".claudio/settings.json").exists());
}

#[test]
#[serial]
fn test_init_quiet_dry_run() {
    let env = TestEnvironment::new();
    std::fs::create_dir_all(env.project_dir.join(".git")).unwrap();

    // Run init with --quiet and --dry-run
    let output = run_claudio(
        &env.project_dir,
        &["init", "--scope", "project", "--quiet", "--dry-run"],
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "Init --quiet --dry-run failed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify output is suppressed
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty() || stderr.trim().is_empty(),
        "Expected completely silent output, got: {}",
        stderr
    );

    // Verify nothing was created
    assert!(!env.project_dir.join(".claudio").exists());
}

#[test]
#[serial]
fn test_init_force_creates_backup() {
    let env = TestEnvironment::new();
    std::fs::create_dir_all(env.project_dir.join(".git")).unwrap();

    // First init
    run_claudio(
        &env.project_dir,
        &["init", "--scope", "project"],
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
    );

    // Corrupt the settings file
    let settings_path = env.project_dir.join(".claudio").join("settings.json");
    std::fs::write(&settings_path, r#"{"corrupted": true}"#).unwrap();

    // Run init with --force
    let output = run_claudio(
        &env.project_dir,
        &["init", "--scope", "project", "--force"],
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "Init --force failed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify backup was created (with timestamp)
    let backup_files = list_backup_files(&env.project_dir.join(".claudio"));
    assert!(
        !backup_files.is_empty(),
        "Backup file should exist, found files: {:?}",
        std::fs::read_dir(env.project_dir.join(".claudio"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect::<Vec<_>>()
    );

    // Verify backup contains corrupted data
    let backup_files = list_backup_files(&env.project_dir.join(".claudio"));
    let backup_path = &backup_files[0];
    let backup_content = std::fs::read_to_string(backup_path).unwrap();
    assert!(
        backup_content.contains("corrupted"),
        "Backup should contain original corrupted data"
    );

    // Verify settings.json was restored with valid JSON
    let settings_content = std::fs::read_to_string(&settings_path).unwrap();
    assert!(
        serde_json::from_str::<serde_json::Value>(&settings_content).is_ok(),
        "Restored settings should be valid JSON"
    );
}

#[test]
#[serial]
fn test_init_force_dry_run_does_not_create_files() {
    let env = TestEnvironment::new();
    std::fs::create_dir_all(env.project_dir.join(".git")).unwrap();

    // First init
    run_claudio(
        &env.project_dir,
        &["init", "--scope", "project"],
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
    );

    // Corrupt the settings file
    let settings_path = env.project_dir.join(".claudio").join("settings.json");
    std::fs::write(&settings_path, r#"{"corrupted": true}"#).unwrap();

    // Record the current backup file count
    let backup_count_before = std::fs::read_dir(env.project_dir.join(".claudio"))
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".backup")
        })
        .count();

    // Run init with --force --dry-run
    let output = run_claudio(
        &env.project_dir,
        &["init", "--scope", "project", "--force", "--dry-run"],
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "Init --force --dry-run failed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify no new backup was created
    let backup_count_after = std::fs::read_dir(env.project_dir.join(".claudio"))
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".backup")
        })
        .count();
    assert_eq!(
        backup_count_after, backup_count_before,
        "No new backup should be created in dry-run mode"
    );

    // Verify settings.json still contains corrupted data
    let settings_content = std::fs::read_to_string(&settings_path).unwrap();
    assert!(
        settings_content.contains("corrupted"),
        "Settings should still be corrupted in dry-run mode"
    );

    // Verify dry-run message is shown
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("dry-run")
            || stderr.contains("exists")
            || stderr.contains("would add")
            || stderr.contains("backup"),
        "Expected dry-run related message in stderr: {}",
        stderr
    );
}

#[test]
#[serial]
fn test_preset_list_json_output() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.create_user_config();

    // Create test presets in both scopes
    env.write_project_preset(
        "project-preset",
        r#"{
  "name": "project-preset",
  "description": "Project test preset",
  "env": {
    "TEST_VAR": "secret_value",
    "ANOTHER_VAR": "another_secret"
  },
  "args": ["--verbose"],
  "extends": null
}"#,
    );

    env.write_user_preset(
        "user-preset",
        r#"{
  "name": "user-preset",
  "description": "User test preset",
  "env": {
    "USER_VAR": "user_secret"
  },
  "prompt": "test prompt"
}"#,
    );

    // Set default preset
    let settings_path = env.project_dir.join(".claudio").join("settings.json");
    std::fs::write(
        &settings_path,
        r#"{
  "default_preset": "project-preset",
  "color": "cyan",
  "no_color": false,
  "require_preset": false,
  "ignore_user_presets": false,
  "dry_run_show_env": true,
  "claude_executable": "claude",
  "presets": []
}"#,
    )
    .unwrap();

    // Run preset list with --json flag
    let output = run_claudio(
        &env.project_dir,
        &["preset", "list", "--json"],
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "Preset list --json command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify JSON output is valid
    let json: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(
        json.is_ok(),
        "Preset list JSON output is not valid JSON:\n{}",
        stdout
    );

    let json_obj = json.unwrap();

    // Verify top-level structure
    assert!(
        json_obj.get("items").is_some(),
        "Expected 'items' field in JSON"
    );
    assert!(
        json_obj.get("default_preset").is_some(),
        "Expected 'default_preset' field in JSON"
    );
    assert!(
        json_obj.get("scopes").is_some(),
        "Expected 'scopes' field in JSON"
    );
    assert!(
        json_obj.get("invalid_files").is_some(),
        "Expected 'invalid_files' field in JSON"
    );

    // Verify default_preset is set correctly
    assert_eq!(
        json_obj.get("default_preset").and_then(|v| v.as_str()),
        Some("project-preset"),
        "Expected default_preset to be 'project-preset'"
    );

    // Verify items array
    let items = json_obj
        .get("items")
        .and_then(|v| v.as_array())
        .expect("Expected items to be an array");

    assert!(
        items.len() >= 2,
        "Expected at least 2 presets (project and user)"
    );

    // Find project-preset in items
    let project_preset = items
        .iter()
        .find(|item| item.get("name").and_then(|v| v.as_str()) == Some("project-preset"))
        .expect("Expected to find project-preset in items");

    // Verify project-preset fields
    assert_eq!(
        project_preset.get("description").and_then(|v| v.as_str()),
        Some("Project test preset")
    );
    assert_eq!(
        project_preset.get("scope").and_then(|v| v.as_str()),
        Some("project")
    );
    assert_eq!(
        project_preset.get("source").and_then(|v| v.as_str()),
        Some("file")
    );
    assert_eq!(
        project_preset.get("is_default").and_then(|v| v.as_bool()),
        Some(true)
    );

    // Verify filepath is present
    assert!(
        project_preset.get("filepath").is_some(),
        "Expected filepath to be present"
    );

    // CRITICAL: Verify env_keys are present but NOT env values (security check)
    let env_keys = project_preset
        .get("env_keys")
        .and_then(|v| v.as_array())
        .expect("Expected env_keys array");

    assert!(
        env_keys.iter().any(|k| k.as_str() == Some("TEST_VAR")),
        "Expected TEST_VAR in env_keys"
    );
    assert!(
        env_keys.iter().any(|k| k.as_str() == Some("ANOTHER_VAR")),
        "Expected ANOTHER_VAR in env_keys"
    );

    // CRITICAL SECURITY CHECK: Ensure no env VALUES are leaked
    let json_str = serde_json::to_string(&json_obj).unwrap();
    assert!(
        !json_str.contains("secret_value"),
        "ERROR: Env value 'secret_value' leaked in JSON output"
    );
    assert!(
        !json_str.contains("another_secret"),
        "ERROR: Env value 'another_secret' leaked in JSON output"
    );
    assert!(
        !json_str.contains("user_secret"),
        "ERROR: Env value 'user_secret' leaked in JSON output"
    );

    // Verify args array
    let args = project_preset
        .get("args")
        .and_then(|v| v.as_array())
        .expect("Expected args array");
    assert!(
        args.iter().any(|a| a.as_str() == Some("--verbose")),
        "Expected --verbose in args"
    );

    // Find user-preset in items
    let user_preset = items
        .iter()
        .find(|item| item.get("name").and_then(|v| v.as_str()) == Some("user-preset"))
        .expect("Expected to find user-preset in items");

    // Verify user-preset fields
    assert_eq!(
        user_preset.get("scope").and_then(|v| v.as_str()),
        Some("user")
    );
    assert_eq!(
        user_preset.get("is_default").and_then(|v| v.as_bool()),
        Some(false),
        "User preset should NOT be marked as default"
    );

    // Verify prompt field is present for user-preset
    assert_eq!(
        user_preset.get("prompt").and_then(|v| v.as_str()),
        Some("test prompt")
    );
}

#[test]
#[serial]
fn test_preset_list_json_respects_fields() {
    let env = TestEnvironment::new();
    env.create_project_config();
    env.create_user_config();

    env.write_project_preset(
        "project-preset",
        r#"{
  "name": "project-preset",
  "description": "Project test preset",
  "env": {
    "TEST_VAR": "secret_value"
  },
  "args": ["--verbose"]
}"#,
    );

    env.write_user_preset(
        "user-preset",
        r#"{
  "name": "user-preset",
  "description": "User test preset",
  "env": {
    "USER_VAR": "user_secret"
  },
  "prompt": "test prompt"
}"#,
    );

    let output = run_claudio(
        &env.project_dir,
        &["preset", "list", "--json", "--fields", "name,filepath"],
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "Preset list --json --fields command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Expected valid JSON output");

    let items = json
        .get("items")
        .and_then(|v| v.as_array())
        .expect("Expected items to be an array");

    let project_preset = items
        .iter()
        .find(|item| item.get("name").and_then(|v| v.as_str()) == Some("project-preset"))
        .expect("Expected to find project-preset in items");

    // Only name + filepath should be present (plus metadata like scope/source/is_default).
    assert!(project_preset.get("filepath").is_some());
    assert!(project_preset.get("description").is_none());
    assert!(project_preset.get("env_keys").is_none());
    assert!(project_preset.get("args").is_none());
    assert!(project_preset.get("prompt").is_none());
}

#[test]
#[serial]
fn test_preset_list_auto_scope_without_project_root_marks_user_scope() {
    let env = TestEnvironment::new();
    env.create_user_config();

    env.write_user_preset(
        "user-only",
        r#"{
  "name": "user-only",
  "description": "User preset"
}"#,
    );

    // Run from a directory without `.claudio/` or `.git/` so `Scope::Auto` resolves to user-only.
    let output = run_claudio(
        env.temp_dir.path(),
        &["preset", "list", "--json"],
        &[(CLAUDIO_HOME_DIR_ENV_VAR, env.home_dir.to_str().unwrap())],
    );

    assert!(
        output.status.success(),
        "Preset list --json command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Expected valid JSON output");

    let items = json
        .get("items")
        .and_then(|v| v.as_array())
        .expect("Expected items to be an array");

    let user_preset = items
        .iter()
        .find(|item| item.get("name").and_then(|v| v.as_str()) == Some("user-only"))
        .expect("Expected to find user-only in items");

    assert_eq!(
        user_preset.get("scope").and_then(|v| v.as_str()),
        Some("user")
    );

    let scopes = json
        .get("scopes")
        .and_then(|v| v.as_array())
        .expect("Expected scopes to be an array");

    assert!(
        scopes.iter().any(|s| s.as_str() == Some("user")),
        "Expected 'user' in scopes"
    );
    assert!(
        !scopes.iter().any(|s| s.as_str() == Some("project")),
        "Did not expect 'project' in scopes"
    );
}
