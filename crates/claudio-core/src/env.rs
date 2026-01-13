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
    let mut result = String::with_capacity(value.len());
    let mut last_match = 0;

    for caps in VAR_REGEX.captures_iter(value) {
        let m = caps.get(0).expect("regex capture 0 is the full match");
        result.push_str(&value[last_match..m.start()]);

        let var_name = &caps[1];
        let var_value = get_required_env_var(var_name)?;
        result.push_str(&var_value);

        last_match = m.end();
    }

    result.push_str(&value[last_match..]);
    Ok(result)
}
