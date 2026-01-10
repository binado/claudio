use crate::commands::{build_resolver_config, effective_read_scope};
use anyhow::{Context, Result};
use claudio_core::preset::resolver;
use claudio_core::preset::store::PresetStore;
use claudio_core::preset::types::PresetSource;
use claudio_core::scope::Scope;
use claudio_core::settings::loader as settings_loader;

pub fn show(
    preset_name: &str,
    scope: Scope,
    resolved: bool,
    json_only: bool,
    claude_executable_cli: Option<&str>,
) -> Result<()> {
    let effective_scope = effective_read_scope(scope);

    // Resolve claude executable with precedence: CLI > Env > Settings > Default
    let claude_executable = claude_executable_cli
        .map(|s| s.to_string())
        .or_else(|| std::env::var("CLAUDIO_CLAUDE_EXECUTABLE").ok())
        .or_else(|| {
            settings_loader::resolve_claude_executable(effective_scope)
                .ok()
                .flatten()
        })
        .unwrap_or_else(|| "claude".to_string());

    // Create a PresetStore for scope-aware lookups
    let store = PresetStore::new(effective_scope)?;

    // Find the preset using PresetStore (validates name, respects scope)
    let located = store
        .find_required(preset_name)
        .with_context(|| format!("Failed to find preset: {}", preset_name))?;

    if json_only {
        let json = serde_json::to_string_pretty(&located.preset)?;
        println!("{}", json);
        return Ok(());
    }

    // Format location string based on source
    let location_str = match &located.source {
        PresetSource::Inline => "(inline preset from settings)".to_string(),
        PresetSource::File(path) => path.display().to_string(),
    };

    if resolved {
        let resolver_cfg = build_resolver_config(effective_scope);

        let resolved_preset = resolver::resolve_inheritance_with_store(
            &located.preset,
            located.source,
            &store,
            &resolver_cfg,
        )
        .with_context(|| format!("Failed to resolve preset: {}", preset_name))?;

        println!("Configuration: {} (resolved)", preset_name);
        println!("Location: {}\n", location_str);

        println!("Resolved Environment Variables:");
        if resolved_preset.env.is_empty() {
            println!("  (none)");
        } else {
            for (key, value) in &resolved_preset.env {
                println!("  {}={}", key, value);
            }
        }

        if let Some(settings) = &resolved_preset.settings {
            println!("\nResolved Settings:");
            let settings_json = serde_json::to_string_pretty(settings)?;
            for line in settings_json.lines() {
                println!("  {}", line);
            }
        }

        if let Some(prompt) = &resolved_preset.prompt {
            println!("\nPrompt:");
            println!("  {}", prompt);
        }

        println!("\nResolved CLI Arguments:");
        if resolved_preset.args.is_empty() {
            println!("  (none)");
        } else {
            for arg in &resolved_preset.args {
                println!("  {}", arg);
            }
        }

        println!("\nCommand that would be executed:");
        let args_str = resolved_preset.args.join(" ");
        let args_suffix = if args_str.is_empty() {
            String::new()
        } else {
            format!(" {}", args_str)
        };
        if resolved_preset.settings.is_some() {
            println!(
                "  {} {} --settings <temp-file>",
                claude_executable, args_suffix
            );
        } else {
            println!("  {}{}", claude_executable, args_suffix);
        }
    } else {
        println!("Configuration: {}", preset_name);
        println!("Location: {}\n", location_str);

        let json = serde_json::to_string_pretty(&located.preset)?;
        println!("{}", json);
    }

    Ok(())
}
