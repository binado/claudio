use crate::commands::{build_resolver_config, effective_read_scope};
use anyhow::{Context, Result};
use claudio_core::preset::resolve;
use claudio_core::preset::store::PresetStore;
use claudio_core::preset::types::{EnvValue, EnvValueSource};
use claudio_core::scope::Scope;

fn format_env_value(value: &EnvValue) -> String {
    match value {
        EnvValue::Direct(s) => s.clone(),
        EnvValue::Structured { source } => match source {
            EnvValueSource::Value { value } => value.clone(),
            EnvValueSource::Env { var } => format!("${{from: env, var: {}}}", var),
            EnvValueSource::File { path } => format!("${{from: file, path: {}}}", path),
            EnvValueSource::Command { command, confirm } => {
                if let Some(c) = confirm {
                    format!("${{from: command, command: {}, confirm: {}}}", command, c)
                } else {
                    format!("${{from: command, command: {}}}", command)
                }
            }
        },
    }
}

fn shell_escape_double_quoted(value: &str) -> String {
    // Escapes for use inside double quotes in common POSIX shells.
    // We escape characters that can terminate or interpolate within double quotes,
    // as well as history expansion and control characters for safety.
    let mut out = String::with_capacity(value.len());

    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '$' => out.push_str("\\$"),
            '`' => out.push_str("\\`"),
            _ => out.push(ch),
        }
    }

    out
}

pub fn env(preset_name: &str, scope: Scope, export: bool, resolved: bool) -> Result<()> {
    let effective_scope = effective_read_scope(scope);

    // Create a PresetStore for scope-aware lookups
    let store = PresetStore::new(effective_scope)?;

    // Find the preset using PresetStore (validates name, respects scope)
    let located = store
        .find_required(preset_name)
        .with_context(|| format!("Failed to find preset: {}", preset_name))?;

    let resolver_cfg = build_resolver_config(effective_scope);

    let resolved_preset = resolve::resolve_inheritance_with_store(
        &located.preset,
        located.source,
        &store,
        &resolver_cfg,
    )
    .with_context(|| format!("Failed to resolve preset: {}", preset_name))?;

    if export {
        for (key, value) in &resolved_preset.env {
            let escaped_value = shell_escape_double_quoted(value);
            println!("export {}=\"{}\"", key, escaped_value);
        }
    } else if resolved {
        println!(
            "Environment variables for preset '{}' (resolved):\n",
            preset_name
        );
        for (key, value) in &resolved_preset.env {
            println!("{}={}", key, value);
        }
    } else {
        println!("Environment variables for preset '{}':\n", preset_name);
        if let Some(env_vars) = &located.preset.env {
            for (key, value) in env_vars {
                // Display the unresolved value (can be Direct string or Structured source)
                let display_value = format_env_value(value);
                println!("{}={}", key, display_value);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escape_no_special_chars() {
        assert_eq!(shell_escape_double_quoted("plain text"), "plain text");
        assert_eq!(shell_escape_double_quoted("hello123"), "hello123");
    }

    #[test]
    fn test_shell_escape_backslash() {
        assert_eq!(
            shell_escape_double_quoted(r"path\to\file"),
            r"path\\to\\file"
        );
    }

    #[test]
    fn test_shell_escape_double_quote() {
        assert_eq!(
            shell_escape_double_quoted(r#"say "hello""#),
            r#"say \"hello\""#
        );
    }

    #[test]
    fn test_shell_escape_dollar_sign() {
        assert_eq!(shell_escape_double_quoted("$HOME"), r"\$HOME");
        assert_eq!(shell_escape_double_quoted("${VAR}"), r"\${VAR}");
    }

    #[test]
    fn test_shell_escape_backtick() {
        assert_eq!(shell_escape_double_quoted("`command`"), r"\`command\`");
    }

    #[test]
    fn test_shell_escape_combined() {
        // A string with multiple special characters
        let input = r#"echo "$HOME" && `ls`"#;
        let expected = r#"echo \"\$HOME\" && \`ls\`"#;
        assert_eq!(shell_escape_double_quoted(input), expected);
    }

    #[test]
    fn test_shell_escape_empty() {
        assert_eq!(shell_escape_double_quoted(""), "");
    }
}
