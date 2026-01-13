use crate::env::CLAUDIO_HOME_DIR_ENV_VAR;
use anyhow::{Result, bail};
use directories::BaseDirs;
use std::path::{Path, PathBuf};

/// Get the claudio home directory, respecting `CLAUDIO_HOME_DIR` environment variable.
///
/// This function checks for the `CLAUDIO_HOME_DIR` environment variable first.
/// If set, it must be an absolute path pointing to an existing directory.
/// If not set, falls back to the user's home directory from the OS.
pub fn get_claudio_home() -> Result<PathBuf> {
    if let Ok(custom_home) = std::env::var(CLAUDIO_HOME_DIR_ENV_VAR) {
        let path = PathBuf::from(&custom_home);

        if !path.is_absolute() {
            bail!(
                "{} must be an absolute path, got: {}",
                CLAUDIO_HOME_DIR_ENV_VAR,
                custom_home
            );
        }

        if !path.exists() {
            bail!(
                "{} path does not exist: {}",
                CLAUDIO_HOME_DIR_ENV_VAR,
                path.display()
            );
        }

        if !path.is_dir() {
            bail!(
                "{} must be a directory, got: {}",
                CLAUDIO_HOME_DIR_ENV_VAR,
                path.display()
            );
        }

        Ok(path)
    } else {
        let home = BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Failed to determine home directory"))?
            .home_dir()
            .to_owned();
        Ok(home)
    }
}

/// Expand `~` and `~/...` to the OS home directory.
///
/// This intentionally does **not** respect `CLAUDIO_HOME_DIR`.
pub fn expand_tilde_os_home(path: &str) -> Result<PathBuf> {
    if path == "~" || path.starts_with("~/") {
        let home = BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Failed to determine home directory"))?
            .home_dir()
            .to_owned();
        Ok(home.join(path.strip_prefix("~/").unwrap_or("")))
    } else {
        Ok(PathBuf::from(path))
    }
}

/// Find the project root by searching upward from a start directory.
///
/// If `start` is `None`, uses the current directory.
/// A project root is a directory containing either `.claudio/` or `.git/`.
pub fn find_project_root(start: Option<&Path>) -> Option<PathBuf> {
    let start = match start {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().ok()?,
    };

    let mut dir: &Path = &start;

    loop {
        if dir.join(".claudio").is_dir() || dir.join(".git").is_dir() {
            return Some(dir.to_path_buf());
        }

        match dir.parent() {
            Some(parent) => dir = parent,
            None => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_get_claudio_home() {
        // Test 1: Valid environment variable with existing directory
        {
            let temp_dir = tempfile::tempdir().unwrap();
            let temp_path = temp_dir.path().to_owned();

            unsafe {
                env::set_var(CLAUDIO_HOME_DIR_ENV_VAR, &temp_path);
            }

            let result = get_claudio_home();
            assert!(result.is_ok(), "should succeed with valid directory");
            assert_eq!(result.unwrap(), temp_path);
        }

        // Test 2: Without environment variable (fallback to home directory)
        {
            unsafe {
                env::remove_var(CLAUDIO_HOME_DIR_ENV_VAR);
            }

            let result = get_claudio_home();
            assert!(result.is_ok(), "should succeed without env var");

            let home = result.unwrap();
            assert!(home.exists(), "home directory should exist");
            assert!(home.is_absolute(), "home directory should be absolute");
        }

        // Test 3: Nonexistent path
        {
            unsafe {
                env::set_var(CLAUDIO_HOME_DIR_ENV_VAR, "/nonexistent/path/to/directory");
            }

            let result = get_claudio_home();
            assert!(result.is_err(), "should fail with nonexistent path");
            assert!(
                result.unwrap_err().to_string().contains("does not exist"),
                "error should mention path does not exist"
            );
        }

        // Test 4: Relative path
        {
            unsafe {
                env::set_var(CLAUDIO_HOME_DIR_ENV_VAR, "./relative/path");
            }

            let result = get_claudio_home();
            assert!(result.is_err(), "should fail with relative path");
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("must be an absolute path"),
                "error should mention absolute path requirement"
            );
        }

        // Test 5: File instead of directory
        {
            let temp_file = tempfile::NamedTempFile::new().unwrap();
            let file_path = temp_file.path().to_owned();

            unsafe {
                env::set_var(CLAUDIO_HOME_DIR_ENV_VAR, &file_path);
            }

            let result = get_claudio_home();
            assert!(result.is_err(), "should fail when path is a file");
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("must be a directory"),
                "error should mention directory requirement"
            );
        }

        unsafe {
            env::remove_var(CLAUDIO_HOME_DIR_ENV_VAR);
        }
    }
}
