use crate::commands::{build_resolver_config, effective_read_scope};
use anyhow::{Context, Result};
use claudio_core::preset::resolver;
use claudio_core::preset::store::PresetStore;
use claudio_core::scope::Scope;
use claudio_core::settings::loader as settings_loader;
use serde::Serialize;
use serde_json::json;
use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::process::{Command, ExitCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    name: String,
    status: CheckStatus,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct DoctorReport {
    checks: Vec<CheckResult>,
    summary: DoctorSummary,
}

#[derive(Debug, Serialize)]
pub struct DoctorSummary {
    pass: usize,
    warn: usize,
    fail: usize,
}

#[derive(Debug, Clone)]
struct LoadedPreset {
    preset: claudio_core::preset::types::Preset,
    source: claudio_core::preset::types::PresetSource,
    origin_scope: Option<Scope>,
    path: Option<std::path::PathBuf>,
}

impl DoctorReport {
    fn new() -> Self {
        Self {
            checks: Vec::new(),
            summary: DoctorSummary {
                pass: 0,
                warn: 0,
                fail: 0,
            },
        }
    }

    fn add_check(&mut self, result: CheckResult) {
        match result.status {
            CheckStatus::Pass => self.summary.pass += 1,
            CheckStatus::Warn => self.summary.warn += 1,
            CheckStatus::Fail => self.summary.fail += 1,
        }
        self.checks.push(result);
    }

    fn exit_code(&self) -> u8 {
        if self.summary.fail > 0 {
            2
        } else if self.summary.warn > 0 {
            1
        } else {
            0
        }
    }
}

pub fn doctor(
    scope: Scope,
    json_output: bool,
    verbose: bool,
    claude_executable_cli: Option<&str>,
) -> Result<ExitCode> {
    let effective_scope = effective_read_scope(scope);

    let claude_executable =
        crate::commands::util::resolve_claude_executable(claude_executable_cli, effective_scope);

    let mut report = DoctorReport::new();

    // Check 1: Claude Code executable
    check_claude_executable(&mut report, &claude_executable);

    // Check 2: Settings files
    check_settings(&mut report, scope);

    // Check 3: Preset directories
    check_preset_directories(&mut report, effective_scope);

    // Preset store + discovery (shared)
    let (store, loaded_presets) = match load_presets(&mut report, effective_scope) {
        Ok(Some(v)) => v,
        Ok(None) => {
            // Preset store/discovery failed; still emit env-var check result for consistency.
            check_environment_variables_skipped(&mut report, verbose);

            // Output results
            if json_output {
                return match output_json(&report) {
                    Ok(exit_code) => Ok(exit_code),
                    Err(err) => {
                        let err = err.context("Doctor JSON output failed");

                        let fallback = json!({
                            "error": err.to_string(),
                            "exit_code": 2,
                        });
                        match serde_json::to_string_pretty(&fallback)
                            .or_else(|_| serde_json::to_string(&fallback))
                        {
                            Ok(fallback_json) => println!("{fallback_json}"),
                            Err(_) => {
                                let hard_fallback = r#"{
  "error": "Failed to serialize doctor report as JSON",
  "exit_code": 2
}"#;
                                println!("{hard_fallback}");
                            }
                        }
                        Ok(ExitCode::from(2))
                    }
                };
            }

            output_human_readable(&report);
            return Ok(ExitCode::from(report.exit_code()));
        }
        Err(e) => return Err(e),
    };

    // Check 4: Preset validity (files + inline)
    check_preset_validity(&mut report, &loaded_presets);

    // Check 5: Inheritance resolution (files + inline)
    check_inheritance(&mut report, effective_scope, &store, &loaded_presets);

    // Check 6: Environment variables
    check_environment_variables(
        &mut report,
        effective_scope,
        &store,
        &loaded_presets,
        verbose,
    );

    // Output results
    if json_output {
        match output_json(&report) {
            Ok(exit_code) => Ok(exit_code),
            Err(err) => {
                let err = err.context("Doctor JSON output failed");

                let fallback = json!({
                    "error": err.to_string(),
                    "exit_code": 2,
                });
                match serde_json::to_string_pretty(&fallback)
                    .or_else(|_| serde_json::to_string(&fallback))
                {
                    Ok(fallback_json) => println!("{fallback_json}"),
                    Err(_) => {
                        let hard_fallback = r#"{
  "error": "Failed to serialize doctor report as JSON",
  "exit_code": 2
}"#;
                        println!("{hard_fallback}");
                    }
                }
                Ok(ExitCode::from(2))
            }
        }
    } else {
        output_human_readable(&report);
        Ok(ExitCode::from(report.exit_code()))
    }
}

fn check_claude_executable(report: &mut DoctorReport, executable: &str) {
    let status = match Command::new(executable).arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                CheckStatus::Pass
            } else {
                CheckStatus::Fail
            }
        }
        Err(_) => CheckStatus::Fail,
    };

    let (message, details) = if status == CheckStatus::Pass {
        (
            "Claude Code executable found".to_string(),
            Some(json!({"executable": executable})),
        )
    } else {
        (
            format!(
                "Claude Code executable '{}' not found or not executable",
                executable
            ),
            Some(
                json!({"executable": executable, "suggestion": "Install Claude Code or configure the correct executable path"}),
            ),
        )
    };

    report.add_check(CheckResult {
        name: "claude_executable".to_string(),
        status,
        message,
        details,
    });
}

fn check_settings(report: &mut DoctorReport, scope: Scope) {
    match scope {
        Scope::Auto => {
            check_settings_scoped(report, Scope::Project);
            check_settings_scoped(report, Scope::User);
        }
        Scope::Project | Scope::User => {
            check_settings_scoped(report, scope);
        }
    }
}

fn check_settings_scoped(report: &mut DoctorReport, scope: Scope) {
    let scope_str = match scope {
        Scope::Project => "project",
        Scope::User => "user",
        Scope::Auto => unreachable!("check_settings_scoped should not be called with Scope::Auto"),
    };

    match settings_loader::load_settings(scope) {
        Ok(Some(settings)) => match settings.validate(scope) {
            Ok(_) => {
                report.add_check(CheckResult {
                    name: format!("settings_valid_{}", scope_str),
                    status: CheckStatus::Pass,
                    message: format!("{} settings file is valid", scope_str),
                    details: None,
                });
            }
            Err(e) => {
                report.add_check(CheckResult {
                    name: format!("settings_valid_{}", scope_str),
                    status: CheckStatus::Fail,
                    message: format!("{} settings validation failed: {}", scope_str, e),
                    details: None,
                });
            }
        },
        Ok(None) => {
            report.add_check(CheckResult {
                name: format!("settings_present_{}", scope_str),
                status: CheckStatus::Warn,
                message: format!("No {} settings file found", scope_str),
                details: None,
            });
        }
        Err(e) => {
            report.add_check(CheckResult {
                name: format!("settings_parse_{}", scope_str),
                status: CheckStatus::Fail,
                message: format!("Failed to parse {} settings: {}", scope_str, e),
                details: None,
            });
        }
    }
}

fn check_preset_directories(report: &mut DoctorReport, scope: Scope) {
    use claudio_core::preset::loader;

    let dirs_to_check: Vec<(&str, Result<Vec<std::path::PathBuf>, _>)> = match scope {
        Scope::Auto => {
            vec![
                ("project", loader::get_preset_dirs_scoped(Scope::Project)),
                ("user", loader::get_preset_dirs_scoped(Scope::User)),
            ]
        }
        Scope::Project => {
            vec![("project", loader::get_preset_dirs_scoped(Scope::Project))]
        }
        Scope::User => {
            vec![("user", loader::get_preset_dirs_scoped(Scope::User))]
        }
    };

    for (scope_name, dirs_result) in dirs_to_check {
        match dirs_result {
            Ok(dirs) => {
                if dirs.is_empty() {
                    report.add_check(CheckResult {
                        name: format!("preset_dir_{}", scope_name),
                        status: CheckStatus::Fail,
                        message: format!(
                            "No preset directory paths returned for {} scope",
                            scope_name
                        ),
                        details: None,
                    });
                    continue;
                }

                let mut any_missing = false;
                let mut dir_details = Vec::new();
                for dir in dirs {
                    let exists = dir.exists() && dir.is_dir();
                    if !exists {
                        any_missing = true;
                    }
                    dir_details.push(json!({
                        "path": dir.display().to_string(),
                        "exists": exists,
                    }));
                }

                let status = if any_missing {
                    CheckStatus::Warn
                } else {
                    CheckStatus::Pass
                };

                let message = if any_missing {
                    format!(
                        "Some {} preset directories are missing (optional)",
                        scope_name
                    )
                } else {
                    format!("{} preset directories exist", scope_name)
                };

                report.add_check(CheckResult {
                    name: format!("preset_dir_{}", scope_name),
                    status,
                    message,
                    details: Some(json!({"directories": dir_details})),
                });
            }
            Err(e) => {
                report.add_check(CheckResult {
                    name: format!("preset_dir_{}", scope_name),
                    status: CheckStatus::Fail,
                    message: format!(
                        "Failed to get preset directories for {} scope: {}",
                        scope_name, e
                    ),
                    details: None,
                });
            }
        }
    }
}

fn load_presets(
    report: &mut DoctorReport,
    scope: Scope,
) -> Result<Option<(PresetStore, Vec<LoadedPreset>)>> {
    let store = match PresetStore::new(scope) {
        Ok(s) => s,
        Err(e) => {
            report.add_check(CheckResult {
                name: "preset_store".to_string(),
                status: CheckStatus::Fail,
                message: format!("Failed to create preset store: {}", e),
                details: None,
            });
            return Ok(None);
        }
    };

    let paths = match store.discover_all() {
        Ok(p) => p,
        Err(e) => {
            report.add_check(CheckResult {
                name: "presets_discover".to_string(),
                status: CheckStatus::Fail,
                message: format!("Failed to discover presets: {}", e),
                details: None,
            });
            return Ok(None);
        }
    };

    let mut loaded = Vec::new();

    // File presets
    for path in paths {
        match claudio_core::preset::loader::load_preset(&path) {
            Ok(preset) => loaded.push(LoadedPreset {
                preset,
                source: claudio_core::preset::types::PresetSource::File(path.clone()),
                origin_scope: None,
                path: Some(path),
            }),
            Err(e) => {
                let path_lossy = path.to_string_lossy();
                let mut hasher = DefaultHasher::new();
                path_lossy.hash(&mut hasher);
                let path_hash = format!("{:016x}", hasher.finish());

                let identifier = match path.file_stem().and_then(|s| s.to_str()) {
                    Some(stem) => format!("{}_{}", stem, path_hash),
                    None => path_hash,
                };
                report.add_check(CheckResult {
                    name: format!("preset_parse_{}", identifier),
                    status: CheckStatus::Fail,
                    message: format!("Failed to parse preset file: {}", e),
                    details: Some(json!({"path": path.display().to_string()})),
                });
            }
        }
    }

    // Inline presets (from settings)
    for (origin, preset) in settings_loader::load_inline_presets_scoped(scope)? {
        loaded.push(LoadedPreset {
            preset,
            source: claudio_core::preset::types::PresetSource::Inline,
            origin_scope: Some(origin),
            path: None,
        });
    }

    Ok(Some((store, loaded)))
}

fn check_preset_validity(report: &mut DoctorReport, presets: &[LoadedPreset]) {
    if presets.is_empty() {
        report.add_check(CheckResult {
            name: "presets_found".to_string(),
            status: CheckStatus::Warn,
            message: "No presets found".to_string(),
            details: None,
        });
        return;
    }

    for loaded in presets {
        let validation = loaded.preset.validate(loaded.path.as_deref());
        let (status, message, details) = if !validation.errors.is_empty() {
            let error_msg = validation.errors.join("; ");
            (
                CheckStatus::Fail,
                format!(
                    "Preset '{}' validation failed: {}",
                    loaded.preset.name, error_msg
                ),
                Some(json!({
                    "errors": validation.errors,
                    "warnings": validation.warnings,
                })),
            )
        } else if !validation.warnings.is_empty() {
            (
                CheckStatus::Warn,
                format!(
                    "Preset '{}' is valid but has warnings: {}",
                    loaded.preset.name,
                    validation.warnings.join("; ")
                ),
                Some(json!({"warnings": validation.warnings})),
            )
        } else {
            (
                CheckStatus::Pass,
                format!("Preset '{}' is valid", loaded.preset.name),
                None,
            )
        };

        let name = match loaded.origin_scope {
            Some(Scope::Project) => format!("preset_valid_inline_project_{}", loaded.preset.name),
            Some(Scope::User) => format!("preset_valid_inline_user_{}", loaded.preset.name),
            Some(Scope::Auto) => unreachable!("Inline origin should not be Scope::Auto"),
            None => format!("preset_valid_{}", loaded.preset.name),
        };

        let details = match (&loaded.origin_scope, &loaded.path, details) {
            (Some(origin), None, Some(mut d)) => {
                if let Some(obj) = d.as_object_mut() {
                    obj.insert(
                        "origin".to_string(),
                        json!(match origin {
                            Scope::Project => "project",
                            Scope::User => "user",
                            Scope::Auto => "auto",
                        }),
                    );
                }
                Some(d)
            }
            (Some(origin), None, None) => Some(json!({
                "origin": match origin {
                    Scope::Project => "project",
                    Scope::User => "user",
                    Scope::Auto => "auto",
                }
            })),
            (None, Some(path), Some(mut d)) => {
                if let Some(obj) = d.as_object_mut() {
                    obj.insert("path".to_string(), json!(path.display().to_string()));
                }
                Some(d)
            }
            (None, Some(path), None) => Some(json!({"path": path.display().to_string()})),
            (_, _, d) => d,
        };

        report.add_check(CheckResult {
            name,
            status,
            message,
            details,
        });
    }
}

fn check_inheritance(
    report: &mut DoctorReport,
    scope: Scope,
    store: &PresetStore,
    presets: &[LoadedPreset],
) {
    let resolver_cfg = build_resolver_config(scope);

    let mut checked_any = false;
    for loaded in presets {
        if loaded.preset.extends.is_none() {
            continue;
        }
        checked_any = true;

        let extends = loaded.preset.extends.clone();
        match resolver::resolve_inheritance_with_store(
            &loaded.preset,
            loaded.source.clone(),
            store,
            &resolver_cfg,
        ) {
            Ok(_) => {
                report.add_check(CheckResult {
                    name: format!("inheritance_{}", loaded.preset.name),
                    status: CheckStatus::Pass,
                    message: format!(
                        "Preset '{}' inheritance resolves correctly",
                        loaded.preset.name
                    ),
                    details: None,
                });
            }
            Err(e) => {
                let error_str = e.to_string();
                let is_circular = error_str.contains("Circular");
                report.add_check(CheckResult {
                    name: format!("inheritance_{}", loaded.preset.name),
                    status: CheckStatus::Fail,
                    message: format!(
                        "Preset '{}' inheritance failed: {}",
                        loaded.preset.name, error_str
                    ),
                    details: Some(json!({
                        "is_circular": is_circular,
                        "extends": extends,
                        "origin": loaded.origin_scope.map(|s| match s {
                            Scope::Project => "project",
                            Scope::User => "user",
                            Scope::Auto => "auto",
                        }),
                        "path": loaded.path.as_ref().map(|p| p.display().to_string()),
                    })),
                });
            }
        }
    }

    if !checked_any {
        report.add_check(CheckResult {
            name: "inheritance_none".to_string(),
            status: CheckStatus::Pass,
            message: "No presets with inheritance found".to_string(),
            details: None,
        });
    }
}

fn check_environment_variables_skipped(report: &mut DoctorReport, verbose: bool) {
    if verbose {
        return;
    }
    report.add_check(CheckResult {
        name: "env_vars_skipped".to_string(),
        status: CheckStatus::Pass,
        message: "Environment variable checks skipped (run with --verbose)".to_string(),
        details: None,
    });
}

fn check_environment_variables(
    report: &mut DoctorReport,
    _scope: Scope,
    _store: &PresetStore,
    presets: &[LoadedPreset],
    verbose: bool,
) {
    if !verbose {
        check_environment_variables_skipped(report, verbose);
        return;
    }

    if presets.is_empty() {
        report.add_check(CheckResult {
            name: "env_vars_none".to_string(),
            status: CheckStatus::Pass,
            message: "No presets found to check environment variables".to_string(),
            details: None,
        });
        return;
    }

    for loaded in presets {
        let Some(env) = &loaded.preset.env else {
            continue;
        };

        let mut missing_env_vars: HashSet<String> = HashSet::new();
        let mut missing_files: Vec<serde_json::Value> = Vec::new();
        let mut command_values: Vec<serde_json::Value> = Vec::new();

        for (key, value) in env {
            match value {
                claudio_core::preset::types::EnvValue::Direct(s) => {
                    for var_name in resolver::extract_variable_references(s) {
                        if std::env::var(&var_name).is_err() {
                            missing_env_vars.insert(var_name);
                        }
                    }
                }
                claudio_core::preset::types::EnvValue::Structured { source } => match source {
                    claudio_core::preset::types::EnvValueSource::Value { .. } => {}
                    claudio_core::preset::types::EnvValueSource::Env { var } => {
                        if std::env::var(var).is_err() {
                            missing_env_vars.insert(var.clone());
                        }
                    }
                    claudio_core::preset::types::EnvValueSource::File { path } => {
                        match resolver::resolve_file_path_for_source(path, &loaded.source) {
                            Ok(resolved_path) => {
                                if std::fs::read_to_string(&resolved_path).is_err() {
                                    missing_files.push(json!({
                                        "key": key,
                                        "path": path,
                                        "resolved_path": resolved_path.display().to_string(),
                                    }));
                                }
                            }
                            Err(e) => {
                                missing_files.push(json!({
                                    "key": key,
                                    "path": path,
                                    "error": e.to_string(),
                                }));
                            }
                        }
                    }
                    claudio_core::preset::types::EnvValueSource::Command { command, confirm } => {
                        command_values.push(json!({
                            "key": key,
                            "command": command,
                            "confirm": confirm,
                        }));
                    }
                },
            }
        }

        if missing_env_vars.is_empty() && missing_files.is_empty() && command_values.is_empty() {
            report.add_check(CheckResult {
                name: format!("env_vars_{}", loaded.preset.name),
                status: CheckStatus::Pass,
                message: format!(
                    "Preset '{}' has no missing environment variables",
                    loaded.preset.name
                ),
                details: loaded
                    .origin_scope
                    .map(|s| json!({"origin": match s { Scope::Project => "project", Scope::User => "user", Scope::Auto => "auto" }})),
            });
            continue;
        }

        let mut missing_env_vars_vec: Vec<String> = missing_env_vars.into_iter().collect();
        missing_env_vars_vec.sort();

        let details = json!({
            "missing_env_vars": missing_env_vars_vec,
            "missing_files": missing_files,
            "command_values": command_values,
            "origin": loaded.origin_scope.map(|s| match s { Scope::Project => "project", Scope::User => "user", Scope::Auto => "auto" }),
            "path": loaded.path.as_ref().map(|p| p.display().to_string()),
        });

        report.add_check(CheckResult {
            name: format!("env_vars_{}", loaded.preset.name),
            status: CheckStatus::Warn,
            message: format!(
                "Preset '{}' has environment values that could not be validated",
                loaded.preset.name
            ),
            details: Some(details),
        });
    }
}

fn output_human_readable(report: &DoctorReport) {
    for check in &report.checks {
        let status_str = match check.status {
            CheckStatus::Pass => "[OK]",
            CheckStatus::Warn => "[WARN]",
            CheckStatus::Fail => "[FAIL]",
        };

        println!("{}: {}", status_str, check.message);
    }

    println!();
    println!(
        "Summary: {} passed, {} warning(s), {} failed",
        report.summary.pass, report.summary.warn, report.summary.fail
    );
    println!("Exit code: {}", report.exit_code());
    println!();
}

fn output_json(report: &DoctorReport) -> Result<ExitCode> {
    let output = json!({
        "checks": report.checks,
        "summary": {
            "pass": report.summary.pass,
            "warn": report.summary.warn,
            "fail": report.summary.fail,
        },
        "exit_code": report.exit_code(),
    });

    let json_str = serde_json::to_string_pretty(&output)
        .context("Failed to serialize doctor report as JSON")?;
    println!("{}", json_str);
    Ok(ExitCode::from(report.exit_code()))
}
