use anyhow::{Context, Result};
use claudio_core::preset::dirs;
use claudio_core::scope::Scope;
use claudio_core::settings::loader as settings_loader;
use claudio_core::settings::types::Settings;
use comfy_table::{ContentArrangement, Table};
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
    quiet: bool,
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

    // Collect output lines for table display
    let mut output_lines: Vec<(String, String)> = Vec::new();

    // Handle preset directory
    let preset_dir = dirs::get_preset_write_dir_scoped(scope)?;
    let preset_dir_exists = preset_dir.exists();

    // Determine the relative path to show (from project root or home)
    let preset_dir_relative = ".claudio/presets".to_string();

    if preset_dir_exists {
        status.preset_dir_existed = true;
        output_lines.push(("exists".to_string(), preset_dir_relative.clone()));
    } else if dry_run {
        output_lines.push(("would-add".to_string(), preset_dir_relative.clone()));
    } else {
        std::fs::create_dir_all(&preset_dir).with_context(|| {
            format!(
                "Failed to create preset directory: {}",
                preset_dir.display()
            )
        })?;
        status.preset_dir_created = true;
        output_lines.push(("created".to_string(), preset_dir_relative.clone()));
    }

    // Handle settings file
    let settings_path = settings_loader::get_settings_path(scope)?;
    let settings_exists = settings_path.exists();

    let settings_relative = ".claudio/settings.json".to_string();

    if settings_exists && force && !dry_run {
        // Create backup with timestamp to prevent overwrites
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut backup_path = settings_path.clone();
        backup_path.set_file_name(format!(
            "{}.{}.backup",
            settings_path.file_name().unwrap().to_string_lossy(),
            timestamp
        ));
        std::fs::copy(&settings_path, &backup_path).with_context(|| {
            format!(
                "Failed to backup settings file to: {}",
                backup_path.display()
            )
        })?;
        status.settings_backed_up = true;
        status.backup_path = Some(backup_path);

        output_lines.push((
            "backup".to_string(),
            format!(
                "{} -> {}",
                ".claudio/settings.json", ".claudio/settings.json.backup"
            ),
        ));

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
        output_lines.push(("created".to_string(), settings_relative.clone()));
    } else if settings_exists {
        status.settings_existed = true;
        output_lines.push(("exists".to_string(), settings_relative.clone()));
    } else if dry_run {
        output_lines.push(("would-add".to_string(), settings_relative.clone()));
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
        output_lines.push(("created".to_string(), settings_relative.clone()));
    }

    // Display output using comfy-table
    if !quiet && !output_lines.is_empty() {
        let mut table = Table::new();
        table.set_content_arrangement(ContentArrangement::Disabled);
        table.load_preset(comfy_table::presets::NOTHING);

        for (status_tag, path) in &output_lines {
            table.add_row(vec![status_tag, &color_config.highlight(path)]);
        }

        eprintln!("{}", table);
    }

    // Print summary
    if !quiet {
        eprintln!();
        if dry_run {
            eprintln!("(Dry-run mode: no changes made)");
        } else if status.settings_created || status.preset_dir_created {
            eprintln!("Initialization complete!");
            if !status.settings_existed && !status.preset_dir_existed {
                eprintln!();
                eprintln!("Next steps:");
                eprintln!(
                    "  - Create your first preset: {}",
                    color_config.highlight("claudio preset add my-preset")
                );
                eprintln!(
                    "  - List available presets:  {}",
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
    }

    Ok(())
}
