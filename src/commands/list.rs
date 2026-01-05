use crate::cli::Scope;
use crate::preset::loader;
use crate::preset::types::Preset;
use anyhow::Result;
use comfy_table::Table;
use std::path::PathBuf;

pub fn list(scope: Scope, preset: Option<&str>, verbose: bool) -> Result<()> {
    if let Some(preset_name) = preset {
        let matches = loader::find_all_presets_scoped(preset_name, scope)?;
        for preset_match in matches {
            println!("{}", preset_match.display());
        }
        return Ok(());
    }

    let preset_dirs = loader::get_preset_dirs_scoped(scope)?;

    for dir in &preset_dirs {
        let mut dir_presets = Vec::<(PathBuf, Preset)>::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json")
                    && let Ok(preset) = loader::load_preset(&path)
                {
                    dir_presets.push((path, preset));
                }
            }
        }

        if dir_presets.is_empty() {
            continue;
        }

        let cwd = std::env::current_dir().ok();
        let source = if cwd.as_ref().is_some_and(|c| dir.starts_with(c)) {
            "Project"
        } else {
            "User"
        };

        println!("{} ({}):", source, dir.display());

        let mut table = Table::new();
        table.load_preset(comfy_table::presets::NOTHING);

        if verbose {
            table.set_header(vec![
                "Name",
                "Description",
                "Filepath",
                "Env Vars",
                "Args",
                "Extends",
            ]);
        } else {
            table.set_header(vec!["Name", "Description", "Filepath"]);
        }

        for (path, preset) in &dir_presets {
            let desc = preset.description.as_deref().unwrap_or("");

            let mut row = vec![
                preset.name.clone(),
                desc.to_string(),
                path.display().to_string(),
            ];

            if verbose {
                let env_count = preset.env.as_ref().map(|e| e.len()).unwrap_or(0);
                let args_count = preset.args.as_ref().map(|a| a.len()).unwrap_or(0);
                let extends = preset.extends.as_deref().unwrap_or("none");

                row.extend([
                    env_count.to_string(),
                    args_count.to_string(),
                    extends.to_string(),
                ]);
            }

            table.add_row(row);
        }

        println!("{}", table);
        println!();
    }

    Ok(())
}
