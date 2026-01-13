use anyhow::{Context, Result};
use regex::Regex;
use std::sync::LazyLock;

pub const CLAUDIO_HOME_DIR_ENV_VAR: &str = "CLAUDIO_HOME_DIR";

static VAR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").expect("Invalid regex pattern"));

fn missing_env_var_message(var: &str) -> String {
    format!(
        "Environment variable '{}' is not set.\n\nSet it with:\n  export {}=your_value",
        var, var
    )
}

/// Extract `${VAR_NAME}` references from a string.
pub fn extract_variable_references(value: &str) -> Vec<String> {
    VAR_REGEX
        .captures_iter(value)
        .map(|caps| caps[1].to_string())
        .collect()
}

/// Read an environment variable with a consistent error message if missing.
pub fn get_required_env_var(var: &str) -> Result<String> {
    std::env::var(var).with_context(|| missing_env_var_message(var))
}

/// Substitute `${VAR}` references in a string using the current process environment.
///
/// Returns an error if any referenced variable is missing.
pub fn substitute_env_vars(value: &str) -> Result<String> {
    // Validate all variables exist before replacement.
    for caps in VAR_REGEX.captures_iter(value) {
        let var_name = &caps[1];
        get_required_env_var(var_name)?;
    }

    let resolved_value = VAR_REGEX.replace_all(value, |caps: &regex::Captures| {
        let var_name = &caps[1];
        std::env::var(var_name).expect("Variable should exist (already validated)")
    });

    Ok(resolved_value.to_string())
}
