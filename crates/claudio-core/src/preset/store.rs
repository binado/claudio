//! Unified preset lookup with scope awareness.
//!
//! `PresetStore` is the single point of entry for all preset lookups.
//! It respects scope, and handles both inline and file-based presets.

use crate::preset::loader;
use crate::preset::types::{Preset, PresetSource};
use crate::scope::Scope;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// A preset along with its origin
#[derive(Debug, Clone)]
pub struct LocatedPreset {
    /// The preset data
    pub preset: Preset,
    /// Where the preset came from
    pub source: PresetSource,
}

/// Unified preset lookup with scope awareness.
///
/// PresetStore is the single point of entry for all preset lookups.
/// It respects scope, and handles both inline and file-based presets.
#[derive(Debug, Clone)]
pub struct PresetStore {
    scope: Scope,
    search_dirs: Vec<PathBuf>,
}

impl PresetStore {
    /// Create a store for the given scope.
    ///
    /// The store will search in directories appropriate for the scope:
    /// - `Scope::Auto`: project first (if exists), then user
    /// - `Scope::Project`: project only
    /// - `Scope::User`: user only
    pub fn new(scope: Scope) -> Result<Self> {
        let search_dirs = loader::get_preset_dirs_scoped(scope)?;
        Ok(Self { scope, search_dirs })
    }

    /// The scope this store was created with
    pub fn scope(&self) -> Scope {
        self.scope
    }

    /// The search directories for this store
    pub fn search_dirs(&self) -> &[PathBuf] {
        &self.search_dirs
    }

    /// Find a preset by name (inline or file-based).
    ///
    /// Returns None if not found.
    /// Always respects scope for all lookups.
    pub fn find(&self, name: &str) -> Result<Option<LocatedPreset>> {
        // First try inline preset from settings
        if let Some(inline_preset) =
            crate::settings::loader::load_inline_preset_scoped(name, self.scope)?
        {
            return Ok(Some(LocatedPreset {
                preset: inline_preset,
                source: PresetSource::Inline,
            }));
        }

        // Then try file-based preset
        for dir in &self.search_dirs {
            for path in loader::candidate_preset_paths_for_name(dir, name) {
                if path.exists() {
                    let preset = loader::load_preset(&path)?;
                    return Ok(Some(LocatedPreset {
                        preset,
                        source: PresetSource::File(path),
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Find a preset, returning an error if not found.
    ///
    /// Convenience wrapper around `find()` that returns an error instead of None.
    pub fn find_required(&self, name: &str) -> Result<LocatedPreset> {
        self.find(name)?.with_context(|| self.not_found_error(name))
    }

    /// Get the path for a preset name (for writing).
    ///
    /// Returns the path in the write directory for this scope.
    pub fn path_for_name(&self, name: &str) -> Result<PathBuf> {
        let write_dir = self.write_dir()?;
        Ok(loader::preset_path_for_name(&write_dir, name))
    }

    /// Get write directory for this scope.
    ///
    /// This is where new presets should be created/edited.
    pub fn write_dir(&self) -> Result<PathBuf> {
        loader::get_preset_write_dir_scoped(self.scope)
    }

    /// Discover all preset paths in scope.
    ///
    /// Returns paths to all .json files in the search directories.
    pub fn discover_all(&self) -> Result<Vec<PathBuf>> {
        loader::discover_presets_scoped(self.scope)
    }

    /// Generate a "not found" error message with helpful context.
    fn not_found_error(&self, name: &str) -> String {
        let locations = self
            .search_dirs
            .iter()
            .map(|p| format!("  - {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "Preset '{}' not found.\n\nSearched in:\n{}\n\nRun `claudio init` to create a presets directory.",
            name, locations
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_env() -> (TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let home_path = temp_dir.path().to_string_lossy().to_string();
        unsafe {
            std::env::set_var("CLAUDIO_HOME_DIR", &home_path);
        }
        (temp_dir, home_path)
    }

    fn cleanup_test_env() {
        unsafe {
            std::env::remove_var("CLAUDIO_HOME_DIR");
        }
    }

    #[test]
    #[serial]
    fn test_find_nonexistent_preset() {
        let (_temp, _) = setup_test_env();
        let store = PresetStore::new(Scope::User).unwrap();

        let result = store.find("nonexistent").unwrap();
        assert!(result.is_none());

        cleanup_test_env();
    }

    #[test]
    #[serial]
    fn test_find_file_preset() {
        let (_temp, home_path) = setup_test_env();

        // Create preset directory and file BEFORE creating the store
        let preset_dir = PathBuf::from(&home_path).join(".claudio").join("presets");
        fs::create_dir_all(&preset_dir).unwrap();

        let preset_path = preset_dir.join("test-preset.json");
        fs::write(
            &preset_path,
            r#"{"name": "test-preset", "description": "A test preset"}"#,
        )
        .unwrap();

        // Now create the store (after directory exists)
        let store = PresetStore::new(Scope::User).unwrap();
        let result = store.find("test-preset").unwrap();

        assert!(result.is_some());
        let located = result.unwrap();
        assert_eq!(located.preset.name, "test-preset");
        assert!(matches!(located.source, PresetSource::File(_)));

        cleanup_test_env();
    }

    #[test]
    #[serial]
    fn test_path_for_name_validates() {
        let (_temp, _) = setup_test_env();
        let store = PresetStore::new(Scope::User).unwrap();

        // Any name should map to a path.
        assert!(store.path_for_name("my-preset").is_ok());
        assert!(store.path_for_name("MyPreset").is_ok());
        assert!(store.path_for_name("a/b").is_ok());
        assert!(store.path_for_name("..").is_ok());

        // Clean names should keep the filename identical.
        let clean_path = store.path_for_name("my-preset").unwrap();
        assert!(
            clean_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with("my-preset.json")
        );

        // Weird names should be encoded.
        let weird_path = store.path_for_name("a/b").unwrap();
        assert!(
            weird_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .contains("%2F")
        );

        cleanup_test_env();
    }

    #[test]
    #[serial]
    fn test_find_rejects_path_traversal() {
        let (_temp, _) = setup_test_env();
        let store = PresetStore::new(Scope::User).unwrap();

        // Finding with arbitrary names is allowed (it just won't exist).
        let result = store.find("../../../etc/passwd").unwrap();
        assert!(result.is_none());

        cleanup_test_env();
    }
}
