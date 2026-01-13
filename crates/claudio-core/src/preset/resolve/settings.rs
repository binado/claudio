use crate::env;
use anyhow::Result;

pub fn resolve_settings_variables(settings: &serde_json::Value) -> Result<serde_json::Value> {
    match settings {
        serde_json::Value::Object(obj) => {
            let mut resolved_map = serde_json::Map::new();
            for (key, value) in obj {
                resolved_map.insert(key.clone(), resolve_settings_value(value)?);
            }
            Ok(serde_json::Value::Object(resolved_map))
        }
        other => Ok(resolve_settings_value(other)?),
    }
}

pub(crate) fn resolve_settings_value(value: &serde_json::Value) -> Result<serde_json::Value> {
    match value {
        serde_json::Value::String(s) => Ok(serde_json::Value::String(env::substitute_env_vars(s)?)),
        serde_json::Value::Array(arr) => {
            let resolved: Result<Vec<_>> = arr.iter().map(resolve_settings_value).collect();
            Ok(serde_json::Value::Array(resolved?))
        }
        serde_json::Value::Object(obj) => {
            let mut resolved_map = serde_json::Map::new();
            for (key, val) in obj {
                resolved_map.insert(key.clone(), resolve_settings_value(val)?);
            }
            Ok(serde_json::Value::Object(resolved_map))
        }
        other => Ok(other.clone()),
    }
}

pub(crate) fn merge_json_settings(
    base: Option<serde_json::Value>,
    derived: Option<serde_json::Value>,
) -> Option<serde_json::Value> {
    match (base, derived) {
        (
            Some(serde_json::Value::Object(mut base_obj)),
            Some(serde_json::Value::Object(derived_obj)),
        ) => {
            for (key, value) in derived_obj {
                let merged_value = if let Some(base_value) = base_obj.get(&key) {
                    merge_json_settings(Some(base_value.clone()), Some(value.clone()))
                        .unwrap_or(value)
                } else {
                    value
                };
                base_obj.insert(key, merged_value);
            }
            Some(serde_json::Value::Object(base_obj))
        }
        (None, derived) => derived,
        (_base, Some(derived)) => Some(derived),
        (base, None) => base,
    }
}
