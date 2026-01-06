//! Common test utilities for integration tests.

#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// A test environment with isolated preset and settings directories.
pub struct TestEnvironment {
    /// The temporary directory (dropped when TestEnvironment is dropped)
    pub temp_dir: TempDir,
    /// Path to user-level presets: <temp>/home/.claudio/presets/
    pub user_preset_dir: PathBuf,
    /// Path to project-level presets: <temp>/project/.claudio/presets/
    pub project_preset_dir: PathBuf,
    /// Path to user-level settings file
    pub user_settings_path: PathBuf,
    /// Path to project-level settings file
    pub project_settings_path: PathBuf,
}

impl TestEnvironment {
    /// Create a new isolated test environment.
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");

        let user_preset_dir = temp_dir.path().join("home/.claudio/presets");
        let project_preset_dir = temp_dir.path().join("project/.claudio/presets");
        let user_settings_path = temp_dir.path().join("home/.claudio/settings.json");
        let project_settings_path = temp_dir.path().join("project/.claudio/settings.json");

        // Create directories
        fs::create_dir_all(&user_preset_dir).expect("Failed to create user preset dir");
        fs::create_dir_all(&project_preset_dir).expect("Failed to create project preset dir");

        Self {
            temp_dir,
            user_preset_dir,
            project_preset_dir,
            user_settings_path,
            project_settings_path,
        }
    }

    /// Create a preset file in the specified scope.
    ///
    /// # Arguments
    /// * `scope` - Either "user" or "project"
    /// * `name` - The preset name (without .json extension)
    /// * `content` - The JSON content of the preset
    pub fn create_preset(&self, scope: &str, name: &str, content: &str) -> PathBuf {
        let dir = match scope {
            "user" => &self.user_preset_dir,
            "project" => &self.project_preset_dir,
            _ => panic!("Invalid scope: {}", scope),
        };

        let path = dir.join(format!("{}.json", name));
        fs::write(&path, content).expect("Failed to write preset file");
        path
    }

    /// Create a settings file in the specified scope.
    ///
    /// # Arguments
    /// * `scope` - Either "user" or "project"
    /// * `content` - The JSON content of the settings
    pub fn create_settings(&self, scope: &str, content: &str) -> PathBuf {
        let path = match scope {
            "user" => &self.user_settings_path,
            "project" => &self.project_settings_path,
            _ => panic!("Invalid scope: {}", scope),
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create settings parent dir");
        }

        fs::write(path, content).expect("Failed to write settings file");
        path.clone()
    }

    /// Get the path to the "home" directory (for CLAUDIO_HOME_DIR)
    pub fn home_path(&self) -> PathBuf {
        self.temp_dir.path().join("home")
    }

    /// Get the path to the "project" directory
    pub fn project_path(&self) -> PathBuf {
        self.temp_dir.path().join("project")
    }
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new()
    }
}
