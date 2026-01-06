use anyhow::{Result, bail};
use directories::BaseDirs;
use std::path::PathBuf;

/// Get the claudio home directory, respecting CLAUDIO_HOME_DIR environment variable.
///
/// This function checks for the `CLAUDIO_HOME_DIR` environment variable first.
/// If set, it must be an absolute path pointing to an existing directory.
/// If not set, falls back to the user's home directory from the system.
///
/// # Returns
///
/// * `Ok(PathBuf)` - The claudio home directory path
/// * `Err` - If the directory doesn't exist, isn't absolute, or home cannot be determined
///
/// # Examples
///
/// ```no_run
/// use claudio::util::get_claudio_home;
/// use std::path::PathBuf;
///
/// // With CLAUDIO_HOME_DIR set
/// unsafe {
///     std::env::set_var("CLAUDIO_HOME_DIR", "/tmp/test-home");
/// }
/// let home = get_claudio_home().unwrap();
/// assert_eq!(home, PathBuf::from("/tmp/test-home"));
///
/// // Without CLAUDIO_HOME_DIR
/// unsafe {
///     std::env::remove_var("CLAUDIO_HOME_DIR");
/// }
/// let home = get_claudio_home().unwrap();
/// // Returns the user's home directory
/// ```
pub fn get_claudio_home() -> Result<PathBuf> {
    if let Ok(custom_home) = std::env::var("CLAUDIO_HOME_DIR") {
        let path = PathBuf::from(&custom_home);

        // Validate that the path is absolute
        if !path.is_absolute() {
            bail!(
                "CLAUDIO_HOME_DIR must be an absolute path, got: {}",
                custom_home
            );
        }

        // Validate that the directory exists
        if !path.exists() {
            bail!("CLAUDIO_HOME_DIR path does not exist: {}", path.display());
        }

        // Validate that it's actually a directory
        if !path.is_dir() {
            bail!(
                "CLAUDIO_HOME_DIR must be a directory, got: {}",
                path.display()
            );
        }

        Ok(path)
    } else {
        // Fallback to the user's home directory
        let home = BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Failed to determine home directory"))?
            .home_dir()
            .to_owned();
        Ok(home)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_claudio_home_with_valid_env_var() {
        // Create a temporary directory
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path().to_owned();

        unsafe {
            env::set_var("CLAUDIO_HOME_DIR", &temp_path);
        }

        let result = get_claudio_home();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), temp_path);

        unsafe {
            env::remove_var("CLAUDIO_HOME_DIR");
        }
    }

    #[test]
    fn test_get_claudio_home_without_env_var() {
        unsafe {
            env::remove_var("CLAUDIO_HOME_DIR");
        }

        let result = get_claudio_home();
        assert!(result.is_ok());

        let home = result.unwrap();
        assert!(home.exists());
        assert!(home.is_absolute());
    }

    #[test]
    fn test_get_claudio_home_with_nonexistent_path() {
        unsafe {
            env::set_var("CLAUDIO_HOME_DIR", "/nonexistent/path/to/directory");
        }

        let result = get_claudio_home();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));

        unsafe {
            env::remove_var("CLAUDIO_HOME_DIR");
        }
    }

    #[test]
    fn test_get_claudio_home_with_relative_path() {
        unsafe {
            env::set_var("CLAUDIO_HOME_DIR", "./relative/path");
        }

        let result = get_claudio_home();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must be an absolute path")
        );

        unsafe {
            env::remove_var("CLAUDIO_HOME_DIR");
        }
    }

    #[test]
    fn test_get_claudio_home_with_file_instead_of_directory() {
        // Create a temporary file
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_owned();

        unsafe {
            env::set_var("CLAUDIO_HOME_DIR", &file_path);
        }

        let result = get_claudio_home();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must be a directory")
        );

        unsafe {
            env::remove_var("CLAUDIO_HOME_DIR");
        }
    }
}
