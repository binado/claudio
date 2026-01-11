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

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    name: String,
    status: String,
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
        match result.status.as_str() {
            "pass" => self.summary.pass += 1,
            "warn" => self.summary.warn += 1,
            "fail" => self.summary.fail += 1,
            _ => {
                eprintln!("Unexpected status: treating as fail");
                self.summary.fail += 1;
            }
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
    check_settings(&mut report, effective_scope);

    // Check 3: Preset directories
    check_preset_directories(&mut report, effective_scope);

    // Check 4: Preset JSON validity
    check_preset_validity(&mut report, effective_scope);

    // Check 5: Inheritance resolution
    check_inheritance(&mut report, effective_scope);

    // Check 6: Environment variables
    check_environment_variables(&mut report, effective_scope, verbose);

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
                "pass"
            } else {
                "fail"
            }
        }
        Err(_) => "fail",
    };

    let (message, details) = if status == "pass" {
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
        status: status.to_string(),
        message,
        details,
    });
}

fn check_settings(report: &mut DoctorReport, scope: Scope) {
    // Check settings files
    match settings_loader::load_settings(scope) {
        Ok(Some(settings)) => match settings.validate(scope) {
            Ok(_) => {
                report.add_check(CheckResult {
                    name: "settings_valid".to_string(),
                    status: "pass".to_string(),
                    message: "Settings file is valid".to_string(),
                    details: None,
                });
            }
            Err(e) => {
                report.add_check(CheckResult {
                    name: "settings_valid".to_string(),
                    status: "fail".to_string(),
                    message: format!("Settings validation failed: {}", e),
                    details: None,
                });
            }
        },
        Ok(None) => {
            report.add_check(CheckResult {
                name: "settings_present".to_string(),
                status: "warn".to_string(),
                message: "No settings file found".to_string(),
                details: None,
            });
        }
        Err(e) => {
            report.add_check(CheckResult {
                name: "settings_parse".to_string(),
                status: "fail".to_string(),
                message: format!("Failed to parse settings: {}", e),
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
                let Some(dir) = dirs.first() else {
                    report.add_check(CheckResult {
                        name: format!("preset_dir_{}", scope_name),
                        status: "fail".to_string(),
                        message: format!(
                            "Failed to get preset directory path for {} scope",
                            scope_name
                        ),
                        details: None,
                    });
                    continue;
                };

                if dir.exists() && dir.is_dir() {
                    report.add_check(CheckResult {
                        name: format!("preset_dir_{}", scope_name),
                        status: "pass".to_string(),
                        message: format!("{} preset directory exists", scope_name),
                        details: Some(json!({"path": dir.display().to_string()})),
                    });
                } else {
                    report.add_check(CheckResult {
                        name: format!("preset_dir_{}", scope_name),
                        status: "warn".to_string(),
                        message: format!(
                            "{} preset directory does not exist (optional)",
                            scope_name
                        ),
                        details: Some(json!({"path": dir.display().to_string()})),
                    });
                }
            }
            Err(e) => {
                report.add_check(CheckResult {
                    name: format!("preset_dir_{}", scope_name),
                    status: "fail".to_string(),
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

fn check_preset_validity(report: &mut DoctorReport, scope: Scope) {
    match PresetStore::new(scope) {
        Ok(store) => match store.discover_all() {
            Ok(paths) => {
                if paths.is_empty() {
                    report.add_check(CheckResult {
                        name: "presets_found".to_string(),
                        status: "warn".to_string(),
                        message: "No presets found".to_string(),
                        details: None,
                    });
                } else {
                    for path in paths {
                        match claudio_core::preset::loader::load_preset(&path) {
                            Ok(preset) => {
                                let validation = preset.validate(Some(&path));
                                let (status, message, details) = if !validation.errors.is_empty() {
                                    let error_msg = validation.errors.join("; ");
                                    (
                                        "fail".to_string(),
                                        format!(
                                            "Preset '{}' validation failed: {}",
                                            preset.name, error_msg
                                        ),
                                        Some(json!({
                                            "errors": validation.errors,
                                            "warnings": validation.warnings,
                                        })),
                                    )
                                } else if !validation.warnings.is_empty() {
                                    (
                                        "warn".to_string(),
                                        format!(
                                            "Preset '{}' is valid but has warnings: {}",
                                            preset.name,
                                            validation.warnings.join("; ")
                                        ),
                                        Some(json!({"warnings": validation.warnings})),
                                    )
                                } else {
                                    (
                                        "pass".to_string(),
                                        format!("Preset '{}' is valid", preset.name),
                                        None,
                                    )
                                };

                                report.add_check(CheckResult {
                                    name: format!("preset_valid_{}", preset.name),
                                    status,
                                    message,
                                    details,
                                });
                            }
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
                                    status: "fail".to_string(),
                                    message: format!("Failed to parse preset file: {}", e),
                                    details: Some(json!({"path": path.display().to_string()})),
                                });
                            }
                        }
                    }
                }
            }
            Err(e) => {
                report.add_check(CheckResult {
                    name: "presets_discover".to_string(),
                    status: "fail".to_string(),
                    message: format!("Failed to discover presets: {}", e),
                    details: None,
                });
            }
        },
        Err(e) => {
            report.add_check(CheckResult {
                name: "preset_store".to_string(),
                status: "fail".to_string(),
                message: format!("Failed to create preset store: {}", e),
                details: None,
            });
        }
    }
}

fn check_inheritance(report: &mut DoctorReport, scope: Scope) {
    match PresetStore::new(scope) {
        Ok(store) => match store.discover_all() {
            Ok(paths) => {
                for path in paths {
                    if let Ok(preset) = claudio_core::preset::loader::load_preset(&path)
                        && preset.extends.is_some()
                    {
                        let resolver_cfg = build_resolver_config(scope);
                        let source = claudio_core::preset::types::PresetSource::File(path);
                        match resolver::resolve_inheritance_with_store(
                            &preset,
                            source,
                            &store,
                            &resolver_cfg,
                        ) {
                            Ok(_) => {
                                report.add_check(CheckResult {
                                    name: format!("inheritance_{}", preset.name),
                                    status: "pass".to_string(),
                                    message: format!(
                                        "Preset '{}' inheritance resolves correctly",
                                        preset.name
                                    ),
                                    details: None,
                                });
                            }
                            Err(e) => {
                                let error_str = e.to_string();
                                let is_circular = error_str.contains("Circular");
                                report.add_check(CheckResult {
                                    name: format!("inheritance_{}", preset.name),
                                    status: "fail".to_string(),
                                    message: format!(
                                        "Preset '{}' inheritance failed: {}",
                                        preset.name, error_str
                                    ),
                                    details: Some(json!({
                                        "is_circular": is_circular,
                                        "extends": preset.extends
                                    })),
                                });
                            }
                        }
                    }
                }
            }
            Err(e) => {
                report.add_check(CheckResult {
                    name: "inheritance_check".to_string(),
                    status: "fail".to_string(),
                    message: format!("Failed to check inheritance: {}", e),
                    details: None,
                });
            }
        },
        Err(e) => {
            report.add_check(CheckResult {
                name: "inheritance_check".to_string(),
                status: "fail".to_string(),
                message: format!("Failed to check inheritance: {}", e),
                details: None,
            });
        }
    }
}

fn check_environment_variables(report: &mut DoctorReport, scope: Scope, verbose: bool) {
    if !verbose {
        return; // Skip this check unless verbose
    }

    if let Ok(store) = PresetStore::new(scope)
        && let Ok(paths) = store.discover_all()
    {
        for path in paths {
            if let Ok(preset) = claudio_core::preset::loader::load_preset(&path)
                && let Some(env) = &preset.env
            {
                let mut missing_vars = HashSet::new();

                for env_value in env.values() {
                    // Check for variable references in direct string values
                    if let claudio_core::preset::types::EnvValue::Direct(s) = env_value {
                        // Simple pattern matching for ${VAR_NAME}
                        extract_missing_vars(s, &mut missing_vars);
                    }
                }

                if !missing_vars.is_empty() {
                    let mut missing_vars_vec: Vec<String> = missing_vars.into_iter().collect();
                    missing_vars_vec.sort();
                    report.add_check(CheckResult {
                        name: format!("env_vars_{}", preset.name),
                        status: "warn".to_string(),
                        message: format!(
                            "Preset '{}' references missing environment variables",
                            preset.name
                        ),
                        details: Some(json!({
                            "missing_variables": missing_vars_vec
                        })),
                    });
                }
            }
        }
    }
}

/// Extract missing environment variable references from a string containing ${VAR_NAME} patterns
fn extract_missing_vars(s: &str, missing_vars: &mut HashSet<String>) {
    let mut iter = s.chars().peekable();
    while let Some(c) = iter.next() {
        if c != '$' {
            continue;
        }

        if iter.peek().copied() != Some('{') {
            continue;
        }

        // consume '{'
        iter.next();

        let mut var_name = String::new();
        while let Some(&ch) = iter.peek() {
            if ch == '}' {
                iter.next();
                break;
            }
            var_name.push(ch);
            iter.next();
        }

        if !var_name.is_empty() && std::env::var(&var_name).is_err() {
            missing_vars.insert(var_name);
        }
    }
}

fn output_human_readable(report: &DoctorReport) {
    for check in &report.checks {
        let status_str = match check.status.as_str() {
            "pass" => "[OK]",
            "warn" => "[WARN]",
            "fail" => "[FAIL]",
            _ => "? UNKNOWN",
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
