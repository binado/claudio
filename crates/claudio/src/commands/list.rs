use crate::color::ColorConfig;
use anyhow::Result;
use claudio_core::preset::types::Preset;
use claudio_core::preset::{dirs, io};
use claudio_core::scope::Scope;
use claudio_core::settings::loader as settings_loader;
use comfy_table::{Attribute, Cell, CellAlignment, ContentArrangement, Table};
use serde::Serialize;
use std::borrow::Cow;

use crate::commands::effective_read_scope;

// JSON output structures
#[derive(Serialize)]
struct PresetListItem {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filepath: Option<String>,
    scope: String,  // "project" | "user"
    source: String, // "file" | "inline"
    is_default: bool,
    // Optional extra fields
    #[serde(skip_serializing_if = "Option::is_none")]
    env_keys: Option<Vec<String>>, // keys only; do not emit values
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extends: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
}

#[derive(Serialize)]
struct InvalidPresetFile {
    path: String,
    error: String,
}

#[derive(Serialize)]
struct PresetListOutput {
    items: Vec<PresetListItem>,
    default_preset: Option<String>,
    scopes: Vec<String>,
    invalid_files: Vec<InvalidPresetFile>,
}

fn should_include_json_field(validated_fields: &Option<Vec<String>>, field: &str) -> bool {
    match validated_fields {
        None => true,
        Some(fields) => fields.iter().any(|f| f == field),
    }
}

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
        .any(|field| field.trim().eq_ignore_ascii_case("all"));
    if has_all {
        return Ok(VALID_FIELDS.iter().map(|s| s.to_string()).collect());
    }

    let mut validated = Vec::new();
    for field in fields {
        let normalized = field.trim().to_lowercase();
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

fn get_display_fields<'a>(validated_fields: &'a Option<Vec<String>>) -> Cow<'a, [String]> {
    if let Some(validated) = validated_fields {
        Cow::Borrowed(validated)
    } else {
        Cow::Owned(vec![
            "name".to_string(),
            "description".to_string(),
            "filepath".to_string(),
        ])
    }
}

fn build_row(
    preset: &Preset,
    filepath_display: &str,
    fields: &[String],
    default_preset: Option<&str>,
    color_config: &ColorConfig,
    env_keys: Option<&[String]>,
) -> Vec<Cell> {
    fields
        .iter()
        .map(|field| match field.as_str() {
            "name" => {
                let name = &preset.name;
                let display_name = if Some(name.as_str()) == default_preset {
                    format!("* {}", name)
                } else {
                    name.clone()
                };
                let mut cell = Cell::new(display_name).set_alignment(CellAlignment::Left);
                if Some(name.as_str()) == default_preset && color_config.enabled {
                    cell = cell
                        .fg(color_config.highlight_color.to_comfy_color())
                        .add_attribute(Attribute::Bold);
                }
                cell
            }
            field_name => {
                let value = match field_name {
                    "description" => preset.description.as_deref().unwrap_or("").to_string(),
                    "filepath" => filepath_display.to_string(),
                    "env" => env_keys.map(|keys| keys.join(", ")).unwrap_or_default(),
                    "args" => preset
                        .args
                        .as_ref()
                        .map(|a| a.join(", "))
                        .unwrap_or_default(),
                    "extends" => preset.extends.as_deref().unwrap_or("none").to_string(),
                    "prompt" => preset.prompt.as_deref().unwrap_or("").to_string(),
                    _ => String::new(),
                };
                Cell::new(value).set_alignment(CellAlignment::Left)
            }
        })
        .collect()
}

pub fn list(
    scope: Scope,
    preset: Option<&str>,
    fields: Option<&[String]>,
    max_width: usize,
    json: bool,
    color_config: &ColorConfig,
) -> Result<()> {
    let effective_scope = effective_read_scope(scope);
    let preset_dirs = dirs::get_preset_dirs_scoped(effective_scope)?;

    if max_width > u16::MAX as usize {
        anyhow::bail!("--max-width must be <= {}", u16::MAX);
    }

    // Load default preset to highlight it
    let default_preset = settings_loader::resolve_default_preset(effective_scope)
        .ok()
        .flatten();

    // Validate fields once before processing
    let validated_fields = if let Some(custom_fields) = fields {
        Some(validate_fields(custom_fields)?)
    } else {
        None
    };

    // Unified data collection
    let mut all_items: Vec<PresetListItem> = Vec::new();
    let mut invalid_files: Vec<InvalidPresetFile> = Vec::new();
    let mut scopes_used: Vec<String> = Vec::new();

    // Collect file-based presets with proper scope labeling
    for (dir_index, dir) in preset_dirs.iter().enumerate() {
        // Determine scope label based on known order (not CWD-based)
        let scope_label = match effective_scope {
            Scope::Project => "project",
            Scope::User => "user",
            Scope::Auto => {
                // For Auto scope, the user dir is always last; project (if present) comes before it.
                let user_dir_index = preset_dirs.len().saturating_sub(1);
                if dir_index == user_dir_index {
                    "user"
                } else {
                    "project"
                }
            }
        };

        if !scopes_used.iter().any(|s| s == scope_label) {
            scopes_used.push(scope_label.to_string());
        }

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.extension().is_none_or(|e| e != "json") {
                    continue;
                }

                match io::load_preset(&path) {
                    Ok(preset_obj) => {
                        if preset.is_none() || preset_obj.name == preset.unwrap() {
                            let is_default = Some(&preset_obj.name) == default_preset.as_ref();

                            all_items.push(PresetListItem {
                                name: preset_obj.name.clone(),
                                description: should_include_json_field(
                                    &validated_fields,
                                    "description",
                                )
                                .then(|| preset_obj.description.clone())
                                .flatten(),
                                filepath: should_include_json_field(&validated_fields, "filepath")
                                    .then(|| path.display().to_string()),
                                scope: scope_label.to_string(),
                                source: "file".to_string(),
                                is_default,
                                env_keys: should_include_json_field(&validated_fields, "env")
                                    .then(|| {
                                        preset_obj
                                            .env
                                            .as_ref()
                                            .map(|e| e.keys().map(|k| k.to_string()).collect())
                                    })
                                    .flatten(),
                                args: should_include_json_field(&validated_fields, "args")
                                    .then(|| preset_obj.args.clone())
                                    .flatten(),
                                extends: should_include_json_field(&validated_fields, "extends")
                                    .then(|| preset_obj.extends.clone())
                                    .flatten(),
                                prompt: should_include_json_field(&validated_fields, "prompt")
                                    .then(|| preset_obj.prompt.clone())
                                    .flatten(),
                            });
                        }
                    }
                    Err(err) => {
                        invalid_files.push(InvalidPresetFile {
                            path: path.display().to_string(),
                            error: err.to_string(),
                        });
                    }
                }
            }
        }
    }

    // Collect inline presets
    for (origin, preset_obj) in settings_loader::load_inline_presets_scoped(effective_scope)? {
        if let Some(filter_name) = preset
            && preset_obj.name != filter_name
        {
            continue;
        }

        let scope_label = match origin {
            Scope::Project => "project",
            Scope::User => "user",
            Scope::Auto => unreachable!(
                "Scope::Auto should not be returned as an origin by load_inline_presets_scoped"
            ),
        };

        if !scopes_used.iter().any(|s| s == scope_label) {
            scopes_used.push(scope_label.to_string());
        }

        let is_default = Some(&preset_obj.name) == default_preset.as_ref();

        all_items.push(PresetListItem {
            name: preset_obj.name.clone(),
            description: should_include_json_field(&validated_fields, "description")
                .then(|| preset_obj.description.clone())
                .flatten(),
            filepath: None,
            scope: scope_label.to_string(),
            source: "inline".to_string(),
            is_default,
            env_keys: should_include_json_field(&validated_fields, "env")
                .then(|| {
                    preset_obj
                        .env
                        .as_ref()
                        .map(|e| e.keys().map(|k| k.to_string()).collect())
                })
                .flatten(),
            args: should_include_json_field(&validated_fields, "args")
                .then(|| preset_obj.args.clone())
                .flatten(),
            extends: should_include_json_field(&validated_fields, "extends")
                .then(|| preset_obj.extends.clone())
                .flatten(),
            prompt: should_include_json_field(&validated_fields, "prompt")
                .then(|| preset_obj.prompt.clone())
                .flatten(),
        });
    }

    // Output based on format
    if json {
        let output = PresetListOutput {
            items: all_items,
            default_preset,
            scopes: scopes_used,
            invalid_files,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // Table output
        let display_fields = get_display_fields(&validated_fields);
        let show_default_legend = display_fields.iter().any(|f| f == "name");

        // Group items by scope and source for display
        let mut default_was_rendered = false;

        // Display file-based presets grouped by scope
        for scope_label in &["project", "user"] {
            let scope_items: Vec<&PresetListItem> = all_items
                .iter()
                .filter(|item| item.source == "file" && item.scope == *scope_label)
                .collect();

            if scope_items.is_empty() {
                continue;
            }

            // Find the corresponding directory for location display
            let dir_path = if *scope_label == "project" {
                preset_dirs.first()
            } else {
                preset_dirs.get(if preset_dirs.len() > 1 { 1 } else { 0 })
            };

            // Enhanced section header
            let header = format!(
                "=== {} Presets ===",
                if *scope_label == "project" {
                    "Project"
                } else {
                    "User"
                }
            );
            if color_config.enabled {
                println!("\n{}", color_config.highlight(&header));
            } else {
                println!("\n{}", header);
            }
            if let Some(dir) = dir_path {
                println!("Location: {}", dir.display());
            }
            println!();

            let mut table = Table::new();
            table.load_preset(comfy_table::presets::NOTHING);

            // Apply max width if specified
            if max_width > 0 {
                table
                    .set_content_arrangement(ContentArrangement::DynamicFullWidth)
                    .set_width(max_width as u16);
            } else {
                table.set_content_arrangement(ContentArrangement::Disabled);
            }

            // Set headers
            let headers: Vec<Cell> = display_fields
                .iter()
                .map(|f| {
                    let cell = Cell::new(field_to_header(f)).set_alignment(CellAlignment::Left);
                    if color_config.enabled {
                        cell.fg(color_config.highlight_color.to_comfy_color())
                            .add_attribute(Attribute::Bold)
                    } else {
                        cell
                    }
                })
                .collect();
            table.set_header(headers);

            // Build rows
            for item in scope_items {
                if show_default_legend && item.is_default {
                    default_was_rendered = true;
                }

                // Convert PresetListItem back to Preset for build_row
                let preset_obj = Preset {
                    name: item.name.clone(),
                    description: item.description.clone(),
                    env: None, // Will use env_keys from item
                    args: item.args.clone(),
                    extends: item.extends.clone(),
                    prompt: item.prompt.clone(),
                    settings: None,
                };

                let filepath_display = item.filepath.as_deref().unwrap_or("");
                let env_keys_slice = item.env_keys.as_deref();
                let row = build_row(
                    &preset_obj,
                    filepath_display,
                    &display_fields,
                    default_preset.as_deref(),
                    color_config,
                    env_keys_slice,
                );
                table.add_row(row);
            }

            for line in table.lines() {
                println!("{}", line);
            }
            println!();
        }

        // Display inline presets grouped by scope
        for scope_label in &["project", "user"] {
            let inline_items: Vec<&PresetListItem> = all_items
                .iter()
                .filter(|item| item.source == "inline" && item.scope == *scope_label)
                .collect();

            if inline_items.is_empty() {
                continue;
            }

            let label = match effective_scope {
                Scope::Project | Scope::User => "Inline",
                Scope::Auto => {
                    if *scope_label == "project" {
                        "Inline (project)"
                    } else {
                        "Inline (user)"
                    }
                }
            };

            // Enhanced section header
            let header = format!("=== {} ===", label);
            if color_config.enabled {
                println!("\n{}", color_config.highlight(&header));
            } else {
                println!("\n{}", header);
            }
            println!();

            let mut table = Table::new();
            table.load_preset(comfy_table::presets::NOTHING);

            // Apply max width if specified
            if max_width > 0 {
                table
                    .set_content_arrangement(ContentArrangement::DynamicFullWidth)
                    .set_width(max_width as u16);
            } else {
                table.set_content_arrangement(ContentArrangement::Disabled);
            }

            let headers: Vec<Cell> = display_fields
                .iter()
                .map(|f| {
                    let cell = Cell::new(field_to_header(f)).set_alignment(CellAlignment::Left);
                    if color_config.enabled {
                        cell.fg(color_config.highlight_color.to_comfy_color())
                            .add_attribute(Attribute::Bold)
                    } else {
                        cell
                    }
                })
                .collect();
            table.set_header(headers);

            for item in inline_items {
                if show_default_legend && item.is_default {
                    default_was_rendered = true;
                }

                let preset_obj = Preset {
                    name: item.name.clone(),
                    description: item.description.clone(),
                    env: None,
                    args: item.args.clone(),
                    extends: item.extends.clone(),
                    prompt: item.prompt.clone(),
                    settings: None,
                };

                let env_keys_slice = item.env_keys.as_deref();
                let row = build_row(
                    &preset_obj,
                    "(inline)",
                    &display_fields,
                    default_preset.as_deref(),
                    color_config,
                    env_keys_slice,
                );
                table.add_row(row);
            }

            for line in table.lines() {
                println!("{}", line);
            }
            println!();
        }

        // Print legend if default preset was shown
        if show_default_legend && default_was_rendered {
            println!("* indicates default preset");
        }

        // Print invalid files summary
        if !invalid_files.is_empty() {
            let msg = if invalid_files.len() == 1 {
                "1 invalid preset file skipped".to_string()
            } else {
                format!("{} invalid preset files skipped", invalid_files.len())
            };
            eprintln!("\n({}, run `claudio doctor` for details)", msg);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────────
    // validate_fields tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_validate_fields_valid() {
        let fields = vec!["name".to_string(), "description".to_string()];
        let result = validate_fields(&fields).unwrap();
        assert_eq!(result, vec!["name", "description"]);
    }

    #[test]
    fn test_validate_fields_all_valid_fields() {
        let fields = vec![
            "name".to_string(),
            "description".to_string(),
            "filepath".to_string(),
            "env".to_string(),
            "args".to_string(),
            "extends".to_string(),
            "prompt".to_string(),
        ];
        let result = validate_fields(&fields).unwrap();
        assert_eq!(result.len(), 7);
    }

    #[test]
    fn test_validate_fields_invalid() {
        let fields = vec!["name".to_string(), "invalid_field".to_string()];
        let result = validate_fields(&fields);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid field"));
    }

    #[test]
    fn test_validate_fields_all_keyword() {
        let fields = vec!["all".to_string()];
        let result = validate_fields(&fields).unwrap();
        assert_eq!(result.len(), 7);
        assert!(result.contains(&"name".to_string()));
        assert!(result.contains(&"prompt".to_string()));
    }

    #[test]
    fn test_validate_fields_all_case_insensitive() {
        let fields = vec!["ALL".to_string()];
        let result = validate_fields(&fields).unwrap();
        assert_eq!(result.len(), 7);

        let fields = vec!["All".to_string()];
        let result = validate_fields(&fields).unwrap();
        assert_eq!(result.len(), 7);
    }

    #[test]
    fn test_validate_fields_case_insensitive() {
        let fields = vec!["NAME".to_string(), "Description".to_string()];
        let result = validate_fields(&fields).unwrap();
        assert_eq!(result, vec!["name", "description"]);
    }

    #[test]
    fn test_validate_fields_whitespace_handling() {
        let fields = vec!["  name  ".to_string(), " description ".to_string()];
        let result = validate_fields(&fields).unwrap();
        assert_eq!(result, vec!["name", "description"]);
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // field_to_header tests
    // ─────────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_field_to_header_known_fields() {
        assert_eq!(field_to_header("name"), "Name");
        assert_eq!(field_to_header("description"), "Description");
        assert_eq!(field_to_header("filepath"), "Filepath");
        assert_eq!(field_to_header("env"), "Env Vars");
        assert_eq!(field_to_header("args"), "Args");
        assert_eq!(field_to_header("extends"), "Extends");
        assert_eq!(field_to_header("prompt"), "Prompt");
    }

    #[test]
    fn test_field_to_header_unknown_field() {
        assert_eq!(field_to_header("unknown"), "unknown");
        assert_eq!(field_to_header("custom_field"), "custom_field");
    }
}
