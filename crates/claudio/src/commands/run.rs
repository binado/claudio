use crate::color::ColorConfig;
use crate::commands::{build_resolver_config, effective_read_scope};
use anyhow::{Context, Result};
use claudio_core::preset::resolver;
use claudio_core::preset::store::PresetStore;
use claudio_core::scope::Scope;
use claudio_core::settings::loader as settings_loader;
use std::io::Write;
use std::process::{Command, ExitCode};

pub fn run(
    preset_name: Option<&str>,
    scope: Scope,
    require_preset_cli: bool,
    dry_run: bool,
    claude_args: &[String],
    claude_executable_cli: Option<&str>,
    color_config: &ColorConfig,
) -> Result<ExitCode> {
    let effective_scope = effective_read_scope(scope);

    let claude_executable =
        crate::commands::util::resolve_claude_executable(claude_executable_cli, effective_scope);

    // Determine effective require_preset: CLI flag overrides settings
    let require_preset = require_preset_cli
        || settings_loader::resolve_require_preset(effective_scope)?.unwrap_or(false);

    // Create a PresetStore for scope-aware lookups
    let store = PresetStore::new(effective_scope)?;

    // Determine which preset to use
    let preset_to_use = match preset_name {
        Some(name) => {
            if !dry_run {
                eprintln!("using preset {}", color_config.highlight(name));
            }
            name.to_string()
        }
        None => {
            // Try to get default preset from settings
            if let Some(default_name) = settings_loader::resolve_default_preset(effective_scope)? {
                if !dry_run {
                    eprintln!(
                        "using preset {} (from settings)",
                        color_config.highlight(&default_name)
                    );
                }
                default_name
            } else if store.find("default")?.is_some() {
                // Fall back to "default" preset file
                if !dry_run {
                    eprintln!("using preset {}", color_config.highlight("default"));
                }
                "default".to_string()
            } else {
                // No default preset found
                if require_preset {
                    anyhow::bail!(
                        "No preset available and 'require_preset' is enabled.\n\n\
                         Specify a preset name, set 'default_preset' in settings, \
                         or create a 'default' preset file."
                    );
                }
                if !dry_run {
                    eprintln!("no preset found, falling back to: {}", claude_executable);
                }
                return run_claude_directly(claude_args, dry_run, &claude_executable);
            }
        }
    };

    // Find the preset using PresetStore (scope-aware, validates name)
    let located = store
        .find_required(&preset_to_use)
        .with_context(|| format!("Failed to find preset: {}", preset_to_use))?;

    let resolver_cfg = build_resolver_config(effective_scope);

    let resolved = resolver::resolve_inheritance_with_store(
        &located.preset,
        located.source,
        &store,
        &resolver_cfg,
    )
    .with_context(|| format!("Failed to resolve preset: {}", preset_to_use))?;

    // Build args list to be used for both command and dry-run output
    let mut args: Vec<String> = Vec::new();

    // Add preset-defined args
    args.extend(resolved.args.clone());

    // Add settings via temporary file if present (more secure than command-line args)
    // In dry-run mode, we keep the file so the printed command is copy/paste-able.
    let mut kept_settings_path: Option<std::path::PathBuf> = None;
    let mut _temp_settings_file: Option<tempfile::NamedTempFile> = None;

    if let Some(settings) = &resolved.settings {
        let settings_json = serde_json::to_string_pretty(settings)
            .context("Failed to serialize settings to JSON")?;

        // Create a temporary file in the system temp directory
        let mut temp_file =
            tempfile::NamedTempFile::new().context("Failed to create temporary settings file")?;

        // Write settings directly through the file handle (avoids race condition)
        temp_file
            .write_all(settings_json.as_bytes())
            .context("Failed to write settings to temporary file")?;

        let temp_path = if dry_run {
            let (file, kept_path) = temp_file
                .keep()
                .context("Failed to keep temporary settings file")?;
            drop(file);
            kept_settings_path = Some(kept_path.clone());
            kept_path
        } else {
            let path = temp_file.path().to_path_buf();
            _temp_settings_file = Some(temp_file);
            path
        };

        args.push("--settings".to_string());
        args.push(temp_path.to_string_lossy().to_string());
    }

    // Add additional claude args
    args.extend(claude_args.iter().cloned());

    // Add prompt if present
    if let Some(prompt) = &resolved.prompt {
        args.push(prompt.clone());
    }

    if dry_run {
        let show_env = settings_loader::resolve_dry_run_show_env(effective_scope)?.unwrap_or(true);
        print_dry_run_command(&resolved.env, &args, show_env, &claude_executable)?;

        if let Some(path) = kept_settings_path {
            eprintln!(
                "# Note: --settings was written to {} (file is kept; delete it when done).",
                path.display()
            );
        }
        return Ok(ExitCode::SUCCESS);
    }

    let mut cmd = Command::new(&claude_executable);

    for (key, value) in &resolved.env {
        cmd.env(key, value);
    }

    for arg in &args {
        cmd.arg(arg);
    }

    execute_claude_command(cmd)
    // _temp_settings_file is dropped here, cleaning up the temporary file
}

fn run_claude_directly(
    claude_args: &[String],
    dry_run: bool,
    claude_executable: &str,
) -> Result<ExitCode> {
    if dry_run {
        print_dry_run_command(
            &std::collections::HashMap::new(),
            claude_args,
            true,
            claude_executable,
        )?;
        return Ok(ExitCode::SUCCESS);
    }

    let mut cmd = Command::new(claude_executable);
    cmd.args(claude_args);
    execute_claude_command(cmd)
}

fn shell_escape(s: &str) -> String {
    if !s.is_empty()
        && s.chars().all(|c| {
            c.is_alphanumeric()
                || c == '_'
                || c == '-'
                || c == '/'
                || c == '.'
                || c == ':'
                || c == '='
        })
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

fn print_dry_run_command(
    env_vars: &std::collections::HashMap<String, String>,
    args: &[String],
    show_env: bool,
    claude_executable: &str,
) -> Result<()> {
    let output = build_dry_run_command(env_vars, args, show_env, claude_executable);
    println!("{}", output);
    std::io::stdout().flush()?;
    Ok(())
}

fn build_dry_run_command(
    env_vars: &std::collections::HashMap<String, String>,
    args: &[String],
    show_env: bool,
    claude_executable: &str,
) -> String {
    let mut output = String::new();

    // Add environment variables if enabled
    if show_env {
        let mut keys: Vec<_> = env_vars.keys().collect();
        keys.sort();
        for key in keys {
            let value = &env_vars[key];
            let escaped_value = shell_escape(value);
            output.push_str(&format!("{}={} ", key, escaped_value));
        }
    }

    // Add command name
    let escaped_executable = shell_escape(claude_executable);
    output.push_str(&escaped_executable);

    // Add arguments
    if !args.is_empty() {
        output.push(' ');
        let args_str = args
            .iter()
            .map(|arg| shell_escape(arg))
            .collect::<Vec<_>>()
            .join(" ");
        output.push_str(&args_str);
    }

    output
}

fn execute_claude_command(mut cmd: Command) -> Result<ExitCode> {
    let status = cmd.status().with_context(|| {
        "Failed to execute 'claude' command.\n\n\
         Make sure Claude Code is installed and available in your PATH.\n\
         See: https://github.com/anthropics/claude-code"
    })?;
    let code = status.code().unwrap_or(1);

    Ok(ExitCode::from(code.clamp(0, 255) as u8))
}

#[cfg(test)]
mod tests {
    use super::{build_dry_run_command, shell_escape};
    use std::collections::HashMap;

    #[test]
    fn shell_escape_allows_common_url_chars() {
        assert_eq!(shell_escape("https://example.com"), "https://example.com");
        assert_eq!(
            shell_escape("http://localhost:8080"),
            "http://localhost:8080"
        );
    }

    #[test]
    fn shell_escape_allows_equals() {
        assert_eq!(shell_escape("key=value"), "key=value");
    }

    #[test]
    fn shell_escape_quotes_spaces() {
        assert_eq!(shell_escape("hello world"), "'hello world'");
    }

    #[test]
    fn shell_escape_escapes_single_quotes() {
        assert_eq!(shell_escape("O'Reilly"), "'O'\\''Reilly'");
    }

    #[test]
    fn build_dry_run_command_no_args_no_env() {
        let env: HashMap<String, String> = HashMap::new();
        let args: Vec<String> = vec![];
        assert_eq!(build_dry_run_command(&env, &args, true, "claude"), "claude");
    }

    #[test]
    fn build_dry_run_command_with_env_vars_sorted() {
        let mut env: HashMap<String, String> = HashMap::new();
        env.insert("B".to_string(), "2".to_string());
        env.insert("A".to_string(), "1".to_string());
        let args: Vec<String> = vec![];
        assert_eq!(
            build_dry_run_command(&env, &args, true, "claude"),
            "A=1 B=2 claude"
        );
    }

    #[test]
    fn build_dry_run_command_show_env_false_omits_env() {
        let mut env: HashMap<String, String> = HashMap::new();
        env.insert("A".to_string(), "1".to_string());
        let args: Vec<String> = vec![];
        assert_eq!(
            build_dry_run_command(&env, &args, false, "claude"),
            "claude"
        );
    }

    #[test]
    fn build_dry_run_command_with_special_chars_in_args() {
        let env: HashMap<String, String> = HashMap::new();
        let args: Vec<String> = vec![
            "--msg".to_string(),
            "hello world".to_string(),
            "O'Reilly".to_string(),
        ];
        assert_eq!(
            build_dry_run_command(&env, &args, false, "claude"),
            "claude --msg 'hello world' 'O'\\''Reilly'"
        );
    }

    #[test]
    fn build_dry_run_command_with_custom_executable() {
        let env: HashMap<String, String> = HashMap::new();
        let args: Vec<String> = vec!["--help".to_string()];
        assert_eq!(
            build_dry_run_command(&env, &args, false, "/usr/local/bin/claude"),
            "/usr/local/bin/claude --help"
        );
    }

    #[test]
    fn build_dry_run_command_with_custom_executable_and_env() {
        let mut env: HashMap<String, String> = HashMap::new();
        env.insert("MY_VAR".to_string(), "value".to_string());
        let args: Vec<String> = vec![];
        assert_eq!(
            build_dry_run_command(&env, &args, true, "/path/to/claude"),
            "MY_VAR=value /path/to/claude"
        );
    }
}
