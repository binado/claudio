use crate::cli::Scope;
use crate::preset::loader;
use crate::preset::types::Preset;
use anyhow::Result;
use comfy_table::Table;
use std::path::PathBuf;

const PROMPT_MAX_LENGTH: usize = 30;

fn validate_fields(fields: &[String]) -> Result<Vec<String>> {
    const VALID_FIELDS: &[&str] = &[
        "name",
        "description",
        "filepath",
        "env",
        "args",
        "extends",
        "prompt",
    ];

    // Handle "all" special case first: if any field is "all" (case-insensitive, trimmed),
    // return all valid fields immediately without validating individual entries.
    let has_all = fields
        .iter()
        .any(|field| field.to_lowercase().trim() == "all");
    if has_all {
        return Ok(VALID_FIELDS.iter().map(|s| s.to_string()).collect());
    }

    let mut validated = Vec::new();
    for field in fields {
        let normalized = field.to_lowercase().trim().to_string();
        if !VALID_FIELDS.contains(&normalized.as_str()) {
            anyhow::bail!(
                "Invalid field '{}'. Valid fields: {}, all",
                field,
                VALID_FIELDS.join(", ")
            );
        }
        validated.push(normalized);
    }

    Ok(validated)
}

fn field_to_header(field: &str) -> &str {
    match field {
        "name" => "Name",
        "description" => "Description",
        "filepath" => "Filepath",
        "env" => "Env Vars",
        "args" => "Args",
        "extends" => "Extends",
        "prompt" => "Prompt",
        _ => field,
    }
}

fn build_row(preset: &Preset, path: &PathBuf, fields: &[String]) -> Vec<String> {
    fields
        .iter()
        .map(|field| match field.as_str() {
            "name" => preset.name.clone(),
            "description" => preset.description.as_deref().unwrap_or("").to_string(),
            "filepath" => path.display().to_string(),
            "env" => preset
                .env
                .as_ref()
                .map(|e| e.keys().map(|k| k.as_str()).collect::<Vec<_>>().join(", "))
                .unwrap_or_default(),
            "args" => preset
                .args
                .as_ref()
                .map(|a| a.join(", "))
                .unwrap_or_default(),
            "extends" => preset.extends.as_deref().unwrap_or("none").to_string(),
            "prompt" => preset
                .prompt
                .as_deref()
                .map(|p| {
                    if p.len() > PROMPT_MAX_LENGTH {
                        format!("{}...", &p[..PROMPT_MAX_LENGTH])
                    } else {
                        p.to_string()
                    }
                })
                .unwrap_or_default(),
            _ => String::new(),
        })
        .collect()
}

pub fn list(
    scope: Scope,
    preset: Option<&str>,
    verbose: bool,
    fields: &Option<Vec<String>>,
) -> Result<()> {
    let preset_dirs = loader::get_preset_dirs_scoped(scope)?;

    // Validate fields once before processing any directories
    let validated_fields = if let Some(custom_fields) = &fields {
        Some(validate_fields(custom_fields)?)
    } else {
        None
    };

    for dir in &preset_dirs {
        let mut dir_presets = Vec::<(PathBuf, Preset)>::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.extension().is_none_or(|e| e != "json") {
                    continue;
                }

                match loader::load_preset(&path) {
                    Ok(preset_obj) => {
                        if preset.is_none() || preset_obj.name == preset.unwrap() {
                            dir_presets.push((path, preset_obj));
                        }
                    }
                    Err(err) => {
                        if verbose {
                            eprintln!("Warning: invalid preset file: {} ({})", path.display(), err);
                        }
                    }
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

        // Determine which fields to display
        let display_fields = if let Some(ref validated) = validated_fields {
            // Use pre-validated custom fields
            validated.clone()
        } else if verbose {
            vec![
                "name".to_string(),
                "description".to_string(),
                "filepath".to_string(),
                "env".to_string(),
                "args".to_string(),
                "extends".to_string(),
            ]
        } else {
            vec![
                "name".to_string(),
                "description".to_string(),
                "filepath".to_string(),
            ]
        };

        // Set headers based on display_fields
        let headers: Vec<&str> = display_fields.iter().map(|f| field_to_header(f)).collect();
        table.set_header(headers);

        // Build rows dynamically based on display_fields
        for (path, preset_obj) in &dir_presets {
            let row = build_row(preset_obj, path, &display_fields);
            table.add_row(row);
        }

        println!("{}", table);
        println!();
    }

    Ok(())
}
