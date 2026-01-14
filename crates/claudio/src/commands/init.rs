use anyhow::{Context, Result};
use claudio_core::paths::{find_project_root, get_claudio_home};
use claudio_core::preset::dirs;
use claudio_core::scope::Scope;
use claudio_core::settings::loader as settings_loader;
use claudio_core::settings::types::Settings;
use comfy_table::{ContentArrangement, Table};
use std::path::{Path, PathBuf};

use crate::color::ColorConfig;

#[derive(Debug, Default)]
struct InitStatus {
    preset_dir_created: bool,
    preset_dir_existed: bool,
    settings_created: bool,
    settings_existed: bool,
    backup_path: Option<PathBuf>,
}

fn format_relative_to_project(path: &Path) -> String {
    path.strip_prefix(std::env::current_dir().unwrap_or_default())
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn format_relative_to_home(path: &Path) -> String {
    let home_dir = get_claudio_home().unwrap_or_else(|_| PathBuf::from("~"));
    match path.strip_prefix(&home_dir) {
        Ok(p) => {
            if p.as_os_str().is_empty() {
                "~".to_string()
            } else {
                format!("~/{}", p.display())
            }
        }
        Err(_) => path.display().to_string(),
    }
}

fn get_relative_display_path(path: &Path, scope: Scope) -> String {
    match scope {
        Scope::Project => format_relative_to_project(path),
        Scope::User => format_relative_to_home(path),
        Scope::Auto => {
            let project_root = find_project_root(None);
            if project_root.as_ref().is_some_and(|p| path.starts_with(p)) {
                format_relative_to_project(path)
            } else {
                format_relative_to_home(path)
            }
        }
    }
}

fn write_default_settings(settings_path: &Path) -> Result<()> {
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create settings directory: {}", parent.display())
        })?;
    }

    let default_settings = Settings::default();
    let json = serde_json::to_string_pretty(&default_settings)
        .context("Failed to serialize settings to JSON")?;

    std::fs::write(settings_path, json)
        .with_context(|| format!("Failed to write settings file: {}", settings_path.display()))
}

pub fn init(
    scope: Scope,
    dry_run: bool,
    quiet: bool,
    force: bool,
    color_config: &ColorConfig,
) -> Result<()> {
    let mut status = InitStatus::default();

    // Collect output lines for table display
    let mut output_lines: Vec<(String, String)> = Vec::new();

    // Handle preset directory
    let preset_dir = dirs::get_preset_write_dir_scoped(scope)?;
    let preset_dir_exists = preset_dir.exists();

    // Determine the relative path to show based on scope
    let preset_dir_relative = get_relative_display_path(&preset_dir, scope);

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

    // Determine the relative path to show based on scope
    let settings_relative = get_relative_display_path(&settings_path, scope);

    if settings_exists && force && !dry_run {
        // Create backup with timestamp to prevent overwrites
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("Failed to get current timestamp")?
            .as_secs();
        let mut backup_path = settings_path.clone();
        let file_name = settings_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Settings path has no file name"))?;
        backup_path.set_file_name(format!(
            "{}.{}.backup",
            file_name.to_string_lossy(),
            timestamp
        ));
        std::fs::copy(&settings_path, &backup_path).with_context(|| {
            format!(
                "Failed to backup settings file to: {}",
                backup_path.display()
            )
        })?;
        status.backup_path = Some(backup_path);

        output_lines.push((
            "backup".to_string(),
            format!(
                "{} -> {}.{}.backup",
                settings_relative, settings_relative, timestamp
            ),
        ));

        write_default_settings(&settings_path)?;
        status.settings_created = true;
        output_lines.push(("created".to_string(), settings_relative.clone()));
    } else if settings_exists && force && dry_run {
        output_lines.push(("would-backup".to_string(), settings_relative.clone()));
    } else if settings_exists {
        status.settings_existed = true;
        output_lines.push(("exists".to_string(), settings_relative.clone()));
    } else if dry_run {
        output_lines.push(("would-add".to_string(), settings_relative.clone()));
    } else {
        write_default_settings(&settings_path)?;
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
