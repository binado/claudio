use crate::preset::resolve::env_values::resolve_variables_with_source;
use crate::preset::resolve::settings::{merge_json_settings, resolve_settings_variables};
use crate::preset::types::{Preset, PresetSource, ResolvedPreset};
use crate::preset::{io, lookup};
use anyhow::{Context, Result};
use std::collections::HashSet;

use super::command::ResolverConfig;

/// Resolve preset inheritance with scope awareness.
///
/// This is the preferred function for resolving presets as it:
/// - Respects the scope when looking up base presets
/// - Properly handles inline presets (no disk lookup for source)
/// - Uses the PresetStore for all lookups
pub fn resolve_inheritance_with_store(
    preset: &Preset,
    source: PresetSource,
    store: &crate::preset::store::PresetStore,
    cfg: &ResolverConfig<'_>,
) -> Result<ResolvedPreset> {
    resolve_inheritance_recursive(
        preset,
        source,
        Some(store),
        cfg,
        &mut HashSet::new(),
        &mut Vec::new(),
    )
}

pub(crate) fn resolve_inheritance_recursive(
    preset: &Preset,
    source: PresetSource,
    store: Option<&crate::preset::store::PresetStore>,
    cfg: &ResolverConfig<'_>,
    visited_set: &mut HashSet<String>,
    visited_vec: &mut Vec<String>,
) -> Result<ResolvedPreset> {
    if visited_set.contains(&preset.name) {
        let chain: Vec<String> = visited_vec
            .iter()
            .cloned()
            .chain(std::iter::once(preset.name.clone()))
            .collect();
        anyhow::bail!(
            "Circular inheritance detected: {}\n\nCheck the 'extends' field in these presets for a circular dependency.",
            chain.join(" -> ")
        );
    }

    visited_set.insert(preset.name.clone());
    visited_vec.push(preset.name.clone());

    let mut env_vars = resolve_variables_with_source(preset, &source, cfg)?;
    let args;
    let settings;

    if let Some(base_name) = &preset.extends {
        let (base_preset, base_source) = if let Some(s) = store {
            let located = s
                .find_required(base_name)
                .with_context(|| format!("Failed to find base preset: {}", base_name))?;
            (located.preset, located.source)
        } else {
            let base_path = lookup::find_preset(base_name)
                .with_context(|| format!("Failed to find base preset: {}", base_name))?;
            let base_preset = io::load_preset(&base_path)
                .with_context(|| format!("Failed to load base preset: {}", base_name))?;
            (base_preset, PresetSource::File(base_path))
        };

        let base_resolved = resolve_inheritance_recursive(
            &base_preset,
            base_source,
            store,
            cfg,
            visited_set,
            visited_vec,
        )?;

        for (key, value) in base_resolved.env {
            env_vars.entry(key).or_insert(value);
        }

        let mut merged_args = base_resolved.args;
        if let Some(preset_args) = &preset.args {
            merged_args.extend_from_slice(preset_args);
        }
        args = merged_args;

        settings = merge_json_settings(base_resolved.settings, preset.settings.clone());
    } else {
        args = preset.args.clone().unwrap_or_default();
        settings = preset.settings.clone();
    }

    visited_set.remove(&preset.name);
    if let Some(popped) = visited_vec.pop() {
        debug_assert_eq!(popped, preset.name);
    }

    let resolved_settings = if let Some(s) = settings {
        Some(resolve_settings_variables(&s)?)
    } else {
        None
    };

    Ok(ResolvedPreset {
        name: preset.name.clone(),
        description: preset.description.clone(),
        prompt: preset.prompt.as_deref().and_then(|p| {
            let t = p.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }),
        env: env_vars,
        args,
        settings: resolved_settings,
        source,
    })
}
