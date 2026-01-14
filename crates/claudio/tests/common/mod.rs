#![allow(dead_code)]

use claudio_core::env::CLAUDIO_HOME_DIR_ENV_VAR;
use std::fs;
use std::path::{Path, PathBuf};

pub struct TestEnvironment {
    pub temp_dir: tempfile::TempDir,
    pub home_dir: PathBuf,
    pub project_dir: PathBuf,
}

impl TestEnvironment {
    pub fn new() -> Self {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let home_dir = temp_dir.path().join("home");
        let project_dir = temp_dir.path().join("project");

        fs::create_dir_all(&home_dir).expect("Failed to create home dir");
        fs::create_dir_all(&project_dir).expect("Failed to create project dir");

        TestEnvironment {
            temp_dir,
            home_dir,
            project_dir,
        }
    }

    pub fn create_project_config(&self) {
        let claudio_dir = self.project_dir.join(".claudio");
        let presets_dir = claudio_dir.join("presets");
        fs::create_dir_all(&presets_dir).expect("Failed to create presets dir");
    }

    pub fn create_user_config(&self) {
        let claudio_dir = self.home_dir.join(".claudio");
        let presets_dir = claudio_dir.join("presets");
        fs::create_dir_all(&presets_dir).expect("Failed to create presets dir");
    }

    pub fn write_project_preset(&self, name: &str, content: &str) {
        let preset_path = self
            .project_dir
            .join(".claudio")
            .join("presets")
            .join(format!("{}.json", name));
        fs::write(&preset_path, content).expect("Failed to write preset file");
    }

    pub fn write_user_preset(&self, name: &str, content: &str) {
        let preset_path = self
            .home_dir
            .join(".claudio")
            .join("presets")
            .join(format!("{}.json", name));
        fs::write(&preset_path, content).expect("Failed to write preset file");
    }

    pub fn write_project_settings(&self, content: &str) {
        let settings_path = self.project_dir.join(".claudio").join("settings.json");
        fs::write(&settings_path, content).expect("Failed to write settings file");
    }

    pub fn write_user_settings(&self, content: &str) {
        let settings_path = self.home_dir.join(".claudio").join("settings.json");
        fs::write(&settings_path, content).expect("Failed to write settings file");
    }

    pub fn setup_env_vars(&self) {
        unsafe {
            std::env::set_var(CLAUDIO_HOME_DIR_ENV_VAR, &self.home_dir);
        }
    }

    pub fn cleanup_env_vars(&self) {
        unsafe {
            std::env::remove_var(CLAUDIO_HOME_DIR_ENV_VAR);
        }
    }
}

pub fn assert_file_exists(path: &Path) {
    assert!(path.exists(), "Expected file {:?} to exist", path);
}

pub fn assert_file_contains(path: &Path, expected_content: &str) {
    let content = fs::read_to_string(path).expect("Failed to read file");
    assert!(
        content.contains(expected_content),
        "Expected file {:?} to contain '{}', but it contained: {}",
        path,
        expected_content,
        content
    );
}
