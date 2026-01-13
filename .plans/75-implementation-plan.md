# Implementation Plan: Improvements to 'claudio preset list' Command

**Issue**: #75
**Status**: Ready for Implementation
**Last Updated**: 2026-01-12

## Overview

This document provides a comprehensive implementation plan for enhancing the `claudio preset list` command with improved UX, clearer interface design, and new functionality for automation use cases.

## Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Scope terminology | Keep `Scope::Auto`, document behavior | Scope enum is shared across commands |
| Verbose flag | Remove entirely | Redundant with `--fields all` |
| Default preset indicator | Append `(*)` to name | Simple, works everywhere |
| JSON output | Include inline presets | Consistency across output modes |
| Error display | Option C: Always show to stderr | Balance between visibility and cleanliness |

## Implementation Phases

### Phase 1: Interface Cleanup (Breaking Changes)

#### 1.1 Remove `--claude-executable` from List Command Context

**Problem**: The `--claude-executable` flag appears as a global option but is only relevant for the `run` command, causing confusion.

**Solution**: Move the flag from global CLI args to `RunArgs` specific context.

**Files to Modify**:
- `crates/claudio/src/cli.rs`

**Changes**:
```rust
// BEFORE
#[derive(Parser)]
#[command(name = "claudio")]
pub struct Cli {
    #[arg(long, global = true)]
    pub claude_executable: Option<String>,
    // ...
}

// AFTER - Remove from Cli, already in RunArgs
#[derive(Parser)]
#[command(name = "claudio")]
pub struct Cli {
    // claude_executable removed from global
    // ...
}
```

**Testing**:
- Verify `claudio preset list --help` doesn't show `--claude-executable`
- Verify `claudio run --help` still shows `--claude-executable`
- Verify `claudio --claude-executable /path run preset` still works

**Complexity**: Low
**Impact**: High (reduces confusion)
**Breaking**: No (only changes help text visibility)

---

#### 1.2 Remove `--verbose` Flag

**Problem**: The `--verbose` flag is redundant with `--fields all` and adds complexity.

**Solution**: Remove the flag entirely and update error display behavior.

**Files to Modify**:
- `crates/claudio/src/cli.rs` (line 124)
- `crates/claudio/src/commands/list.rs` (lines 130-137, 169-171, 301-308)
- `crates/claudio/src/main.rs` (if verbose is passed to list function)

**Changes in `cli.rs`**:
```rust
// BEFORE
List {
    name: Option<String>,
    #[command(flatten)]
    scope: ScopeArgs,
    #[arg(short, long)]
    verbose: bool,  // REMOVE THIS
    #[arg(long, value_delimiter = ',')]
    fields: Option<Vec<String>>,
    // ...
}

// AFTER
List {
    name: Option<String>,
    #[command(flatten)]
    scope: ScopeArgs,
    // verbose removed
    #[arg(long, value_delimiter = ',')]
    fields: Option<Vec<String>>,
    // ...
}
```

**Changes in `list.rs`**:
```rust
// BEFORE
pub fn list(
    scope: Scope,
    preset: Option<&str>,
    verbose: bool,  // REMOVE THIS
    fields: Option<&[String]>,
    prompt_max_length: usize,
    color_config: &ColorConfig,
) -> Result<()>

// AFTER
pub fn list(
    scope: Scope,
    preset: Option<&str>,
    fields: Option<&[String]>,
    prompt_max_length: usize,
    color_config: &ColorConfig,
) -> Result<()>
```

**Error Display Changes** (lines 167-172):
```rust
// BEFORE
Err(err) => {
    skipped_count += 1;
    if verbose {
        eprintln!("Warning: invalid preset file: {} ({})", path.display(), err);
    }
}

// AFTER (Option C: Always show to stderr)
Err(err) => {
    skipped_count += 1;
    eprintln!("Warning: invalid preset file: {} ({})", path.display(), err);
}
```

**Remove verbose-conditional summary** (lines 301-308):
```rust
// BEFORE
if skipped_count > 0 && !verbose {
    let msg = if skipped_count == 1 {
        "1 invalid preset file skipped".to_string()
    } else {
        format!("{} invalid preset files skipped", skipped_count)
    };
    eprintln!("({}, use --verbose to see details)", msg);
}

// AFTER (warnings already shown above, so this block can be removed entirely)
// No summary needed since warnings are always displayed
```

**Update `get_display_fields` function** (lines 62-85):
```rust
// BEFORE
fn get_display_fields<'a>(
    validated_fields: &'a Option<Vec<String>>,
    verbose: bool,
) -> Cow<'a, [String]> {
    if let Some(validated) = validated_fields {
        Cow::Borrowed(validated)
    } else if verbose {
        // Show all fields
    } else {
        // Show default fields
    }
}

// AFTER
fn get_display_fields<'a>(
    validated_fields: &'a Option<Vec<String>>,
) -> Cow<'a, [String]> {
    if let Some(validated) = validated_fields {
        Cow::Borrowed(validated)
    } else {
        // Always show default fields (name, description, filepath)
        Cow::Owned(vec![
            "name".to_string(),
            "description".to_string(),
            "filepath".to_string(),
        ])
    }
}
```

**Update all call sites** (lines 196, 274):
```rust
// BEFORE
let display_fields = get_display_fields(&validated_fields, verbose);

// AFTER
let display_fields = get_display_fields(&validated_fields);
```

**Testing**:
- Verify `claudio preset list --verbose` returns error "unknown flag"
- Verify invalid preset warnings always appear on stderr
- Verify normal output (table) goes to stdout
- Test stderr redirection: `claudio preset list 2>/dev/null` should suppress warnings
- Update unit tests that use verbose parameter

**Complexity**: Medium
**Impact**: High (breaking change, simplifies interface)
**Breaking**: Yes (document in changelog)

---

#### 1.3 Document `--scope auto` Behavior

**Problem**: The `--scope auto` help text doesn't clarify that it lists presets from all scopes.

**Solution**: Update help text in CLI definition.

**Files to Modify**:
- `crates/claudio/src/cli.rs` (lines 8-9, 28-30)

**Changes**:
```rust
// BEFORE
#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum Scope {
    /// Use the default resolution behavior (project shadows user for reads).
    Auto,
    // ...
}

// AFTER
#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum Scope {
    /// Use the default resolution behavior. For 'list': shows presets from all available scopes (project and user). For other commands: project shadows user.
    Auto,
    // ...
}
```

**Alternative**: Add command-specific help text override in `PresetCommands::List`:
```rust
List {
    /// Optional preset name to filter by
    name: Option<String>,

    #[command(flatten)]
    scope: ScopeArgs,
    // Add help text clarification:
    // "Note: --scope auto lists presets from both project and user scopes"
}
```

**Testing**:
- Verify `claudio preset list --help` shows updated description
- Verify actual behavior matches documentation (lists from both scopes)

**Complexity**: Low
**Impact**: Medium (improves clarity)
**Breaking**: No

---

### Phase 2: Enhanced Functionality

#### 2.1 Highlight Default Preset

**Feature**: Append `(*)` to the preset name when it matches the configured default preset.

**Files to Modify**:
- `crates/claudio/src/commands/list.rs`

**Implementation Steps**:

1. **Load default preset from settings** (add after line 138):
```rust
pub fn list(
    scope: Scope,
    preset: Option<&str>,
    fields: Option<&[String]>,
    prompt_max_length: usize,
    color_config: &ColorConfig,
) -> Result<()> {
    let effective_scope = effective_read_scope(scope);
    let preset_dirs = loader::get_preset_dirs_scoped(effective_scope)?;

    // Load default preset name from settings
    let default_preset = match settings_loader::load_settings_scoped(effective_scope) {
        Ok((settings, _)) => settings.default_preset,
        Err(e) => {
            eprintln!("Warning: Could not load settings to determine default preset: {}", e);
            None
        }
    };

    // ... rest of function
}
```

2. **Update `build_row` function signature** (line 87):
```rust
// BEFORE
fn build_row(
    preset: &Preset,
    filepath_display: &str,
    fields: &[String],
    prompt_max_length: usize,
) -> Vec<Cell>

// AFTER
fn build_row(
    preset: &Preset,
    filepath_display: &str,
    fields: &[String],
    prompt_max_length: usize,
    default_preset: Option<&str>,
) -> Vec<Cell>
```

3. **Update name field handling** (lines 96-97):
```rust
let value = match field.as_str() {
    "name" => {
        let mut name = preset.name.clone();
        if default_preset.is_some_and(|default| default == preset.name) {
            name.push_str(" (*)");
        }
        name
    },
    "description" => preset.description.as_deref().unwrap_or("").to_string(),
    // ... rest of fields
}
```

4. **Update all `build_row` call sites** (lines 215-223, 291):
```rust
// Update both locations where build_row is called
let row = build_row(
    preset_obj,
    &filepath_display,
    &display_fields,
    prompt_max_length,
    default_preset.as_deref(),  // NEW PARAMETER
);
```

5. **Add consolidated legend when indicators are shown**:
```rust
// After printing tables, before function return
let show_legend = default_preset.is_some() || !shadowed_presets.is_empty();
if show_legend {
    let mut legend = Vec::new();
    if default_preset.is_some() {
        legend.push("(*) default preset");
    }
    if !shadowed_presets.is_empty() {
        legend.push("(shadowed) hidden by project preset");
    }
    println!("Legend: {}", legend.join(" | "));
    println!();
}
```

**Testing**:
- Set `default_preset` in `.claudio/settings.json`
- Verify preset name shows `(*)` suffix
- Verify `(*)` only appears on the matching preset
- Test with no default preset configured (no `(*)` should appear)
- Test with default preset in different scopes (project vs user)
- Test with `--fields name` (should still show indicator)
- Test inline presets also get `(*)` when they are the default

**Complexity**: Low
**Impact**: High (immediate visual feedback)
**Breaking**: No

---

#### 2.2 Add JSON Output Format

**Feature**: Add `--json` flag for machine-readable output suitable for scripting and automation.

**Files to Modify**:
- `crates/claudio/src/cli.rs`
- `crates/claudio/src/commands/list.rs`

**Changes in `cli.rs`** (line 136):
```rust
List {
    name: Option<String>,
    #[command(flatten)]
    scope: ScopeArgs,
    #[arg(long, value_delimiter = ',')]
    fields: Option<Vec<String>>,
    #[arg(long, default_value_t = 30)]
    prompt_max_length: usize,
    /// Output as JSON for scripting and automation
    #[arg(long)]
    json: bool,  // NEW FLAG
}
```

**Implementation in `list.rs`**:

1. **Add json parameter to function signature**:
```rust
pub fn list(
    scope: Scope,
    preset: Option<&str>,
    fields: Option<&[String]>,
    prompt_max_length: usize,
    color_config: &ColorConfig,
    json: bool,  // NEW PARAMETER
) -> Result<()>
```

2. **Define JSON output structure**:
```rust
use serde::Serialize;

#[derive(Serialize)]
struct JsonPreset {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    source: PresetSource,
    scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    filepath: Option<String>,
    is_default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    env: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extends: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum PresetSource {
    File,
    Inline,
}

#[derive(Serialize)]
struct JsonOutput {
    presets: Vec<JsonPreset>,
}
```

3. **Collect presets for JSON output** (early return after loading all presets):
```rust
pub fn list(
    scope: Scope,
    preset: Option<&str>,
    fields: Option<&[String]>,
    prompt_max_length: usize,
    color_config: &ColorConfig,
    json: bool,
) -> Result<()> {
    let effective_scope = effective_read_scope(scope);
    let preset_dirs = loader::get_preset_dirs_scoped(effective_scope)?;

    let default_preset = match settings_loader::load_settings_scoped(effective_scope) {
        Ok((settings, _)) => settings.default_preset,
        Err(e) => {
            eprintln!("Warning: Could not load settings to determine default preset: {}", e);
            None
        }
    };

    if json {
        return output_json(effective_scope, preset, &preset_dirs, default_preset.as_deref(), validated_fields.as_ref().map(|v| v.as_slice()));
    }

    // ... existing table output code
}
```

4. **Implement `output_json` function**:
```rust
fn output_json(
    effective_scope: Scope,
    filter: Option<&str>,
    preset_dirs: &[PathBuf],
    default_preset: Option<&str>,
    fields: Option<&[String]>,
) -> Result<()> {
    let mut all_presets = Vec::new();

    // Collect file-based presets
    for dir in preset_dirs {
        let cwd = std::env::current_dir().ok();
        let scope_name = if cwd.as_ref().is_some_and(|c| dir.starts_with(c)) {
            "project"
        } else {
            "user"
        };

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_none_or(|e| e != "json") {
                    continue;
                }

                if let Ok(preset) = loader::load_preset(&path) {
                    if filter.is_none() || preset.name == filter.unwrap() {
                        all_presets.push(JsonPreset {
                            name: preset.name.clone(),
                            description: preset.description.clone(),
                            source: PresetSource::File,
                            scope: scope_name.to_string(),
                            filepath: Some(path.display().to_string()),
                            is_default: default_preset.is_some_and(|d| d == preset.name),
                            env: preset.env.clone(),
                            args: preset.args.clone(),
                            extends: preset.extends.clone(),
                            prompt: preset.prompt.clone(),
                        });
                    }
                }
            }
        }
    }

    // Collect inline presets
    for (origin, preset) in settings_loader::load_inline_presets_scoped(effective_scope)? {
        if filter.is_none() || preset.name == filter.unwrap() {
            let scope_name = match origin {
                Scope::Project => "project",
                Scope::User => "user",
                Scope::Auto => unreachable!(),
            };

            all_presets.push(JsonPreset {
                name: preset.name.clone(),
                description: preset.description.clone(),
                source: PresetSource::Inline,
                scope: scope_name.to_string(),
                filepath: None,
                is_default: default_preset.is_some_and(|d| d == preset.name),
                env: preset.env.clone(),
                args: preset.args.clone(),
                extends: preset.extends.clone(),
                prompt: preset.prompt.clone(),
            });
        }
    }

    // Apply fields filter if specified
    let filtered_presets: Vec<JsonPreset> = if let Some(field_list) = fields {
        all_presets.into_iter().filter(|p| {
            field_list.iter().all(|f| match f.as_str() {
                "name" => true,
                "description" => p.description.is_some(),
                "source" => true, // always relevant
                "scope" => true, // always relevant
                "filepath" => p.filepath.is_some(),
                "is_default" => true, // always relevant
                "env" => p.env.is_some(),
                "args" => p.args.is_some(),
                "extends" => p.extends.is_some(),
                "prompt" => p.prompt.is_some(),
                _ => false,
            })
        }).collect()
    } else {
        all_presets
    };

    let output = JsonOutput {
        presets: filtered_presets,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
```

**Testing**:
- Verify `--json` output is valid JSON
- Test JSON with file-based presets only
- Test JSON with inline presets
- Test JSON with both file and inline presets
- Test JSON with `--scope project`, `--scope user`, `--scope auto`
- Test JSON with filter: `claudio preset list my-preset --json`
- Test JSON includes `is_default: true` for default preset
- Test JSON output can be piped to `jq`
- Verify `source` field correctly shows "file" vs "inline"

**Complexity**: Medium
**Impact**: Medium (enables automation)
**Breaking**: No

---

#### 2.3 Improve Scope Grouping Headers

**Feature**: Add clearer section headers when displaying presets from multiple scopes, and indicate inline presets in the output.

**Files to Modify**:
- `crates/claudio/src/commands/list.rs` (lines 182-189)

**Changes**:

```rust
// BEFORE (lines 182-189)
let cwd = std::env::current_dir().ok();
let source = if cwd.as_ref().is_some_and(|c| dir.starts_with(c)) {
    "Project"
} else {
    "User"
};

println!("{} ({}):", source, dir.display());

// AFTER - More prominent headers
let cwd = std::env::current_dir().ok();
let (source, label) = if cwd.as_ref().is_some_and(|c| dir.starts_with(c)) {
    ("Project", "Project Presets")
} else {
    ("User", "User Presets")
};

// Add visual separation
if effective_scope == Scope::Auto {
    println!("\n{}", color_config.highlight(&format!("=== {} ===", label)));
    println!("{}\n", dir.display());
} else {
    println!("{} ({}):", source, dir.display());
}
```

**Inline Preset Indicator in Table Output**:
Add source column or indicator when inline presets are present. Update `build_row` to show source type:

```rust
"source" => {
    let source_type = if filepath.is_none() { "inline" } else { "file" };
    source_type.to_string()
}
```

Or add indicator to name field:
```rust
"name" => {
    let mut name = preset.name.clone();
    if default_preset.is_some_and(|default| default == preset.name) {
        name.push_str(" (*)");
    }
    if filepath.is_none() {
        name.push_str(" [inline]");
    }
    if is_shadowed {
        name.push_str(" (shadowed)");
    }
    name
}
```

**Alternative** (simpler, no color changes):
```rust
let source_label = if cwd.as_ref().is_some_and(|c| dir.starts_with(c)) {
    "Project Presets"
} else {
    "User Presets"
};

println!("\n{}:", source_label);
println!("  Location: {}", dir.display());
println!();
```

**Testing**:
- Test with `--scope auto` (both sections should be clearly separated)
- Test with `--scope project` (only project header)
- Test with `--scope user` (only user header)
- Test with `--no-color` (headers should still be readable)
- Test inline preset `[inline]` indicator appears in table
- Test inline presets show `"source": "inline"` in JSON

**Complexity**: Low
**Impact**: Medium (visual improvement)
**Breaking**: No

---

#### 2.4 Show Shadowed Presets Indicator

**Feature**: When a project preset has the same name as a user preset, mark the user preset as "(shadowed)" since it won't be used. This applies to both file-based and inline presets.

**Files to Modify**:
- `crates/claudio/src/commands/list.rs`

**Implementation**:

1. **Detect shadowing** (after line 149):
```rust
// After collecting all presets, detect which are shadowed
let mut project_preset_names = std::collections::HashSet::new();
let mut user_preset_names = std::collections::HashSet::new();

// First pass: collect preset names by scope from files
for dir in &preset_dirs {
    let cwd = std::env::current_dir().ok();
    let is_project = cwd.as_ref().is_some_and(|c| dir.starts_with(c));

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }

            if let Ok(preset) = loader::load_preset(&path) {
                if is_project {
                    project_preset_names.insert(preset.name.clone());
                } else {
                    user_preset_names.insert(preset.name.clone());
                }
            }
        }
    }
}

// Also check inline presets from settings
for (origin, preset) in settings_loader::load_inline_presets_scoped(effective_scope)? {
    match origin {
        Scope::Project => { project_preset_names.insert(preset.name.clone()); }
        Scope::User => { user_preset_names.insert(preset.name.clone()); }
        Scope::Auto => unreachable!(),
    };
}

// Determine which user presets are shadowed
let shadowed_presets: std::collections::HashSet<_> = user_preset_names
    .intersection(&project_preset_names)
    .cloned()
    .collect();
```

2. **Update `build_row` to accept shadowing info**:
```rust
fn build_row(
    preset: &Preset,
    filepath_display: &str,
    fields: &[String],
    prompt_max_length: usize,
    default_preset: Option<&str>,
    is_shadowed: bool,  // NEW PARAMETER
) -> Vec<Cell>
```

3. **Modify name field to show shadowing**:
```rust
"name" => {
    let mut name = preset.name.clone();
    if default_preset.is_some_and(|default| default == preset.name) {
        name.push_str(" (*)");
    }
    if is_shadowed {
        name.push_str(" (shadowed)");
    }
    name
}
```

4. **Pass shadowing info when building rows**:
```rust
let is_project_dir = cwd.as_ref().is_some_and(|c| dir.starts_with(c));
let is_shadowed = !is_project_dir && shadowed_presets.contains(&preset_obj.name);

let row = build_row(
    preset_obj,
    &filepath_display,
    &display_fields,
    prompt_max_length,
    default_preset.as_deref(),
    is_shadowed,
);
```

5. **Add footer note**:
```rust
// Footer legend is now consolidated in Phase 2.1
```

**Testing**:
- Create preset with same name in both project and user scopes
- Verify user preset shows "(shadowed)" when using `--scope auto`
- Verify no shadowing indicator when using `--scope project` or `--scope user`
- Test with multiple shadowed presets
- Test shadowed preset is still marked as default if configured

**Complexity**: Medium
**Impact**: Medium (helps understand precedence)
**Breaking**: No

---

## Testing Strategy

### Unit Tests

**New tests to add in `list.rs`**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_row_with_default_indicator() {
        let preset = Preset {
            name: "test-preset".to_string(),
            description: Some("Test".to_string()),
            ..Default::default()
        };
        let fields = vec!["name".to_string()];
        let row = build_row(&preset, "/path", &fields, 30, Some("test-preset"), false);

        let name_cell = &row[0];
        assert!(name_cell.content().contains("(*)"));
    }

    #[test]
    fn test_build_row_with_shadowed_indicator() {
        let preset = Preset {
            name: "shadowed".to_string(),
            ..Default::default()
        };
        let fields = vec!["name".to_string()];
        let row = build_row(&preset, "/path", &fields, 30, None, true);

        let name_cell = &row[0];
        assert!(name_cell.content().contains("(shadowed)"));
    }

    #[test]
    fn test_json_output_structure() {
        // Test JSON serialization
        let preset = JsonPreset {
            name: "test".to_string(),
            description: Some("Test preset".to_string()),
            source: PresetSource::File,
            scope: "project".to_string(),
            filepath: Some("/path/to/preset.json".to_string()),
            is_default: true,
            env: None,
            args: None,
            extends: None,
            prompt: None,
        };

        let json = serde_json::to_string(&preset).unwrap();
        assert!(json.contains("\"is_default\":true"));
        assert!(json.contains("\"source\":\"file\""));
    }
}
```

### Integration Tests

Create new integration test file: `crates/claudio/tests/list_command_test.rs`

```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_list_without_verbose_flag() {
    let mut cmd = Command::cargo_bin("claudio").unwrap();
    cmd.arg("preset").arg("list").arg("--verbose");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument '--verbose'"));
}

#[test]
fn test_list_json_output() {
    let mut cmd = Command::cargo_bin("claudio").unwrap();
    cmd.arg("preset").arg("list").arg("--json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"presets\""));
}

#[test]
fn test_inline_preset_in_json_output() {
    // Setup: Create settings with inline preset
    let temp_dir = TempDir::new().unwrap();
    // ... setup inline preset in settings.json

    let mut cmd = Command::cargo_bin("claudio").unwrap();
    cmd.arg("preset").arg("list").arg("--json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"source\":\"inline\""));
}

#[test]
fn test_default_preset_indicator() {
    // Setup: Create temp dir with preset and settings
    let temp_dir = TempDir::new().unwrap();
    // ... setup code

    let mut cmd = Command::cargo_bin("claudio").unwrap();
    cmd.arg("preset").arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("my-preset (*)"));
}

#[test]
fn test_shadowed_preset_indicator() {
    // Setup: Create presets with same name in project and user scopes
    // ... setup code

    let mut cmd = Command::cargo_bin("claudio").unwrap();
    cmd.arg("preset").arg("list").arg("--scope").arg("auto");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("(shadowed)"));
}

#[test]
fn test_inline_preset_shadows_file_preset() {
    // Setup: Create file preset in user scope, inline preset in project scope with same name
    // ... setup code

    let mut cmd = Command::cargo_bin("claudio").unwrap();
    cmd.arg("preset").arg("list").arg("--scope").arg("auto");

    cmd.assert()
        .success()
        .stdout(predicates::str::contains("my-preset (shadowed)"));
}

#[test]
fn test_json_with_empty_presets() {
    // Setup: Create empty temp directories
    let temp_dir = TempDir::new().unwrap();
    // ... setup code

    let mut cmd = Command::cargo_bin("claudio").unwrap();
    cmd.arg("preset").arg("list").arg("--scope").arg("project").arg("--json");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("{\"presets\":[]}"));
}

#[test]
fn test_json_with_fields_filter() {
    let mut cmd = Command::cargo_bin("claudio").unwrap();
    cmd.arg("preset").arg("list").arg("--json").arg("--fields", "name,description");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"description\""))
        .stdout(predicate::str::not().contains("\"env\""));
}

#[test]
fn test_settings_load_failure_warning() {
    // Setup: Create invalid settings file
    let temp_dir = TempDir::new().unwrap();
    std::fs::write(temp_dir.path().join("settings.json"), "invalid json{{{").unwrap();
    // ... setup code

    let mut cmd = Command::cargo_bin("claudio").unwrap();
    cmd.arg("preset").arg("list");

    cmd.assert()
        .success()
        .stderr(predicate::str::contains("Warning"));
}
```

### Manual Testing Checklist

- [ ] `claudio preset list` shows default fields (name, description, filepath)
- [ ] `claudio preset list --fields all` shows all fields
- [ ] `claudio preset list --fields name,env` shows only specified fields
- [ ] `claudio preset list --json` outputs valid JSON
- [ ] Default preset shows `(*)` indicator
- [ ] Shadowed presets show `(shadowed)` indicator
- [ ] Invalid presets show warnings on stderr
- [ ] `claudio preset list 2>/dev/null` suppresses warnings
- [ ] `--scope auto` lists both project and user presets
- [ ] `--scope project` lists only project presets
- [ ] `--scope user` lists only user presets
- [ ] `--no-color` disables colored output
- [ ] Inline presets appear in all output modes
- [ ] Help text clarifies `--scope auto` behavior
- [ ] Legend appears when indicators are shown

### Edge Case Testing

- [ ] Empty preset directories (no presets found)
- [ ] No presets match filter
- [ ] All presets are shadowed
- [ ] Settings file missing or invalid (warning shown, continues)
- [ ] Inline preset with same name as file preset
- [ ] `--json` with `--fields` filters output correctly
- [ ] JSON output with empty preset list
- [ ] `--fields invalid` returns error

---

## Migration Guide

### For Users

**Breaking Changes**:

1. **`--verbose` flag removed**
   - **Before**: `claudio preset list --verbose`
   - **After**: `claudio preset list --fields all`
   - **Migration**: Replace `--verbose` with `--fields all` in scripts

2. **Error display changes**
   - **Before**: Invalid presets only shown with `--verbose`
   - **After**: Invalid presets always shown on stderr
   - **Migration**: Redirect stderr if you want to suppress warnings: `claudio preset list 2>/dev/null`

**New Features**:

1. **Default preset indicator**: Look for `(*)` next to preset names
2. **JSON output**: Use `--json` for scripting: `claudio preset list --json | jq '.presets[0].name'`
3. **Shadowed preset indicator**: Understand which presets are hidden by project-level presets
4. **Legend**: A consolidated legend appears when indicators are shown
5. **Inline preset indicator**: Inline presets show `[inline]` in table output and `"source": "inline"` in JSON

### For Contributors

**API Changes**:

```rust
// OLD
pub fn list(
    scope: Scope,
    preset: Option<&str>,
    verbose: bool,
    fields: Option<&[String]>,
    prompt_max_length: usize,
    color_config: &ColorConfig,
) -> Result<()>

// NEW
pub fn list(
    scope: Scope,
    preset: Option<&str>,
    fields: Option<&[String]>,
    prompt_max_length: usize,
    color_config: &ColorConfig,
    json: bool,
) -> Result<()>
```

---

## Implementation Checklist

### Phase 1: Interface Cleanup
- [ ] Remove `--claude-executable` from list command help (1.1)
- [ ] Remove `--verbose` flag from CLI definition (1.2)
- [ ] Update `list()` function signature (1.2)
- [ ] Remove verbose parameter from `get_display_fields()` (1.2)
- [ ] Update error display to always use stderr (1.2)
- [ ] Remove verbose-conditional error summary (1.2)
- [ ] Update all call sites (1.2)
- [ ] Update help text for `--scope auto` (1.3)
- [ ] Write unit tests for changes (1.2)

### Phase 2: Enhanced Functionality
- [ ] Load default preset from settings (2.1)
- [ ] Update `build_row()` signature with default_preset param (2.1)
- [ ] Add `(*)` indicator logic (2.1)
- [ ] Add consolidated legend (2.1)
- [ ] Test default preset highlighting (2.1)
- [ ] Add `--json` flag to CLI (2.2)
- [ ] Define JSON output structures (2.2)
- [ ] Implement `output_json()` function with fields filtering (2.2)
- [ ] Test JSON output (2.2)
- [ ] Improve scope grouping headers (2.3)
- [ ] Add inline preset indicator (`[inline]`) to table output (2.3)
- [ ] Test header formatting and inline indicators (2.3)
- [ ] Detect shadowed presets including inline presets (2.4)
- [ ] Update `build_row()` with shadowing param (2.4)
- [ ] Add shadowing indicator logic (2.4)
- [ ] Test shadowing indicator (2.4)

### Testing & Documentation
- [ ] Write unit tests
- [ ] Write integration tests
- [ ] Run manual testing checklist
- [ ] Run edge case testing checklist
- [ ] Verify function signatures match current codebase
- [ ] Update CHANGELOG.md with breaking changes
- [ ] Update README.md if needed
- [ ] Update CLAUDE.md design doc

---

## File Modification Summary

| File | Lines Modified | Type | Complexity |
|------|---------------|------|------------|
| `crates/claudio/src/cli.rs` | ~20 | Modified | Low |
| `crates/claudio/src/commands/list.rs` | ~200 | Modified | Medium-High |
| `crates/claudio/tests/list_command_test.rs` | ~120 | New | Medium |
| `CHANGELOG.md` | ~20 | Modified | Low |

**Total estimated LOC**: ~360 lines

---

## Timeline Estimate

| Phase | Estimated Time |
|-------|---------------|
| 1.1 Remove claude-executable | 30 minutes |
| 1.2 Remove verbose flag | 2 hours |
| 1.3 Document scope behavior | 15 minutes |
| 2.1 Default preset highlighting | 1.5 hours |
| 2.2 JSON output | 3.5 hours |
| 2.3 Scope grouping + inline indicators | 1.5 hours |
| 2.4 Shadowed presets | 2.5 hours |
| Testing | 3 hours |
| Documentation | 1 hour |
| **Total** | **~16 hours** |

---

## Notes & Considerations

### Design Decisions

1. **Why remove `--verbose` instead of keeping it?**
   - Reduces interface complexity
   - `--fields all` provides equivalent functionality
   - Simplifies maintenance (one less flag to test)

2. **Why append `(*)` instead of using color?**
   - Works in both colored and non-colored output
   - Compatible with JSON output (though we use `is_default` field there)
   - Simple and universally understood convention

3. **Why Option C for error display?**
   - Balances visibility (users see issues) with cleanliness (stdout is clean)
   - Allows selective suppression with stderr redirection
   - Standard practice for CLI tools

### Future Enhancements

Not included in this iteration but could be added later:

1. **Table width management**: Use comfy-table's width settings instead of `--prompt-max-length`
2. **RGB color support**: Add TrueColor support for custom highlighting colors
3. **Filtering by field values**: e.g., `--filter "env=ANTHROPIC_API_KEY"`
4. **Sorting options**: Sort by name, description, etc.
5. **Export formats**: Add CSV, YAML output formats

### Implementation Notes

**Verify function signatures before implementation**:
- Line 292: `settings_loader::load_settings_scoped` - verify return type and available methods
- Line 512: `settings_loader::load_inline_presets_scoped` - verify function signature
- `Scope::Auto` usage - ensure unreachable!() is appropriate or handle explicitly

**Error handling approach**:
- Settings loading failures show warning and continue (no hard failure)
- Invalid presets show warnings on stderr
- All error paths tested for graceful degradation

---

## References

- **Issue**: #75
- **Planning Document**: `.plans/list-command-improvements.md`
- **Related Files**:
  - `crates/claudio/src/commands/list.rs`
  - `crates/claudio/src/cli.rs`
  - `crates/claudio-core/src/scope.rs`
  - `crates/claudio-core/src/settings/types.rs`
