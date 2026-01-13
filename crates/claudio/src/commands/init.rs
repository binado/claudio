use anyhow::{Context, Result};
use claudio_core::preset::dirs;
use claudio_core::scope::Scope;
use claudio_core::settings::loader as settings_loader;
use claudio_core::settings::types::Settings;
use std::path::PathBuf;

use crate::color::ColorConfig;

#[derive(Debug)]
struct InitStatus {
    preset_dir_created: bool,
    preset_dir_existed: bool,
    settings_created: bool,
    settings_existed: bool,
    settings_backed_up: bool,
    backup_path: Option<PathBuf>,
}

pub fn init(
    scope: Scope,
    dry_run: bool,
    verbose: bool,
    force: bool,
    color_config: &ColorConfig,
) -> Result<()> {
    let mut status = InitStatus {
        preset_dir_created: false,
        preset_dir_existed: false,
        settings_created: false,
        settings_existed: false,
        settings_backed_up: false,
        backup_path: None,
    };

    let scope_name = match scope {
        Scope::Auto => {
            // Detect if we're in a project
            if dirs::get_preset_write_dir_scoped(scope)
                .ok()
                .and_then(|_| {
                    claudio_core::paths::find_project_root(None)?;
                    Some(())
                })
                .is_some()
            {
                "project"
            } else {
                "user"
            }
        }
        Scope::Project => "project",
        Scope::User => "user",
    };

    eprintln!(
        "Initializing claudio in {} scope{}",
        color_config.highlight(scope_name),
        if scope_name == "project" {
            " (git repository detected)"
        } else {
            ""
        }
    );

    if verbose
        && scope_name == "project"
        && let Some(root) = claudio_core::paths::find_project_root(None)
    {
        eprintln!("  Project root: {}", root.display());
    }

    // Handle preset directory
    let preset_dir = dirs::get_preset_write_dir_scoped(scope)?;
    let preset_dir_exists = preset_dir.exists();

    if preset_dir_exists {
        status.preset_dir_existed = true;
        eprintln!(
            "  • Directory exists: {}",
            color_config.highlight(preset_dir.file_name().unwrap().to_string_lossy().as_ref())
        );
    } else if dry_run {
        eprintln!(
            "  ? Would create: {}",
            color_config.highlight(preset_dir.file_name().unwrap().to_string_lossy().as_ref())
        );
    } else {
        std::fs::create_dir_all(&preset_dir).with_context(|| {
            format!(
                "Failed to create preset directory: {}",
                preset_dir.display()
            )
        })?;
        status.preset_dir_created = true;
        eprintln!(
            "  ✓ Created directory: {}",
            color_config.highlight(preset_dir.file_name().unwrap().to_string_lossy().as_ref())
        );
    }

    // Handle settings file
    let settings_path = settings_loader::get_settings_path(scope)?;
    let settings_exists = settings_path.exists();

    if settings_exists && force && !dry_run {
        // Create backup
        let mut backup_path = settings_path.clone();
        backup_path.set_file_name(format!(
            "{}.backup",
            settings_path.file_name().unwrap().to_string_lossy()
        ));
        std::fs::copy(&settings_path, &backup_path).with_context(|| {
            format!(
                "Failed to backup settings file to: {}",
                backup_path.display()
            )
        })?;
        status.settings_backed_up = true;
        status.backup_path = Some(backup_path);

        eprintln!(
            "  ⟳ Backed up:       {} → {}",
            color_config.highlight("settings.json"),
            color_config.highlight("settings.json.backup")
        );

        // Write new settings
        if let Some(parent) = settings_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create settings directory: {}", parent.display())
            })?;
        }

        let default_settings = Settings::default();
        let json = serde_json::to_string_pretty(&default_settings)
            .context("Failed to serialize settings to JSON")?;

        std::fs::write(&settings_path, json).with_context(|| {
            format!("Failed to write settings file: {}", settings_path.display())
        })?;

        status.settings_created = true;
        eprintln!(
            "  ✓ Created settings: {}",
            color_config.highlight("settings.json")
        );
    } else if settings_exists {
        status.settings_existed = true;
        eprintln!(
            "  • Settings exist:   {}",
            color_config.highlight("settings.json")
        );
    } else if dry_run {
        eprintln!(
            "  ? Would create: {}",
            color_config.highlight("settings.json")
        );
    } else {
        // Create settings file
        if let Some(parent) = settings_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create settings directory: {}", parent.display())
            })?;
        }

        let default_settings = Settings::default();
        let json = serde_json::to_string_pretty(&default_settings)
            .context("Failed to serialize settings to JSON")?;

        std::fs::write(&settings_path, json).with_context(|| {
            format!("Failed to write settings file: {}", settings_path.display())
        })?;

        status.settings_created = true;
        eprintln!(
            "  ✓ Created settings: {}",
            color_config.highlight("settings.json")
        );
    }

    // Print summary
    eprintln!();
    if dry_run {
        eprintln!("(Dry-run mode: no changes made)");
    } else if status.settings_created || status.preset_dir_created {
        eprintln!("Initialization complete!");
        if !status.settings_existed && !status.preset_dir_existed {
            eprintln!();
            eprintln!("Next steps:");
            eprintln!(
                "  • Create your first preset: {}",
                color_config.highlight("claudio preset add my-preset")
            );
            eprintln!(
                "  • List available presets:  {}",
                color_config.highlight("claudio preset list")
            );
        }
    } else {
        eprintln!("Already initialized. No changes made.");
    }

    if let Some(backup_path) = &status.backup_path {
        eprintln!();
        eprintln!(
            "Settings file backed up to: {}",
            color_config.highlight(backup_path.to_string_lossy().as_ref())
        );
    }

    Ok(())
}
