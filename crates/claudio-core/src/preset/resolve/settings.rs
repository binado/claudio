use crate::env;
use anyhow::Result;
use serde_json::Value;

/// Recursively resolve `${VAR}` references in a JSON settings value.
pub fn resolve_settings_variables(settings: &Value) -> Result<Value> {
    resolve_value_recursive(settings)
}

/// Internal recursive resolver for JSON values.
///
/// Exposed for fine-grained testing. Production code should use `resolve_settings_variables`.
#[cfg(test)]
pub(crate) fn resolve_settings_value(value: &Value) -> Result<Value> {
    resolve_value_recursive(value)
}

fn resolve_value_recursive(value: &Value) -> Result<Value> {
    match value {
        Value::String(s) => Ok(Value::String(env::substitute_env_vars(s)?)),
        Value::Array(arr) => arr
            .iter()
            .map(resolve_value_recursive)
            .collect::<Result<Vec<_>>>()
            .map(Value::Array),
        Value::Object(obj) => obj
            .iter()
            .map(|(k, v)| resolve_value_recursive(v).map(|resolved| (k.clone(), resolved)))
            .collect::<Result<serde_json::Map<_, _>>>()
            .map(Value::Object),
        other => Ok(other.clone()),
    }
}

/// Deep merge two JSON settings objects.
///
/// - Objects are recursively merged (derived keys override base keys)
/// - Non-object values: derived takes precedence over base
/// - None values are treated as "no value" (the other side wins)
pub(crate) fn merge_json_settings(base: Option<Value>, derived: Option<Value>) -> Option<Value> {
    match (base, derived) {
        (Some(Value::Object(mut base_obj)), Some(Value::Object(derived_obj))) => {
            for (key, derived_value) in derived_obj {
                let merged = match base_obj.get(&key) {
                    Some(base_value) => {
                        merge_json_settings(Some(base_value.clone()), Some(derived_value.clone()))
                            .unwrap_or(derived_value)
                    }
                    None => derived_value,
                };
                base_obj.insert(key, merged);
            }
            Some(Value::Object(base_obj))
        }
        (None, derived) => derived,
        (base, None) => base,
        (_, Some(derived)) => Some(derived),
    }
}
