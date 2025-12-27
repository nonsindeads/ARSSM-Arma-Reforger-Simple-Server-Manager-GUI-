use crate::{models::ServerProfile, storage::AppSettings};

#[derive(Debug)]
pub struct DefaultField {
    pub path: String,
    pub kind: String,
    pub value: String,
}

pub fn apply_default_server_json_settings(
    config: &mut serde_json::Value,
    settings: &AppSettings,
) {
    for field in flatten_defaults(&settings.server_json_defaults) {
        let enabled = settings
            .server_json_enabled
            .get(&field.path)
            .copied()
            .unwrap_or(true);
        if enabled {
            if let Ok(value) = parse_value_by_kind(&field.kind, &field.value) {
                let _ = set_json_path(config, &field.path, value);
            }
        }
    }
}

pub fn apply_profile_overrides(
    config: &mut serde_json::Value,
    profile: &ServerProfile,
) -> Result<(), String> {
    let overrides = if profile.server_json_overrides.is_object() {
        &profile.server_json_overrides
    } else {
        return Ok(());
    };

    for field in flatten_defaults(overrides) {
        let enabled = profile
            .server_json_override_enabled
            .get(&field.path)
            .copied()
            .unwrap_or(false);
        if enabled {
            let value = parse_value_by_kind(&field.kind, &field.value)?;
            set_json_path(config, &field.path, value)?;
        }
    }
    Ok(())
}

pub fn flatten_defaults(value: &serde_json::Value) -> Vec<DefaultField> {
    let mut fields = Vec::new();
    flatten_value(value, "", &mut fields);
    fields
}

fn flatten_value(value: &serde_json::Value, prefix: &str, out: &mut Vec<DefaultField>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                let path = if prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_value(val, &path, out);
            }
        }
        serde_json::Value::Array(list) => {
            let value_string = serde_json::to_string(list).unwrap_or_default();
            out.push(DefaultField {
                path: prefix.to_string(),
                kind: "array".to_string(),
                value: value_string,
            });
        }
        serde_json::Value::String(text) => out.push(DefaultField {
            path: prefix.to_string(),
            kind: "string".to_string(),
            value: text.clone(),
        }),
        serde_json::Value::Number(num) => out.push(DefaultField {
            path: prefix.to_string(),
            kind: "number".to_string(),
            value: num.to_string(),
        }),
        serde_json::Value::Bool(value) => out.push(DefaultField {
            path: prefix.to_string(),
            kind: "bool".to_string(),
            value: value.to_string(),
        }),
        serde_json::Value::Null => out.push(DefaultField {
            path: prefix.to_string(),
            kind: "null".to_string(),
            value: "".to_string(),
        }),
    }
}

pub fn parse_defaults_form(
    form: &std::collections::HashMap<String, String>,
    baseline: &serde_json::Value,
) -> Result<(serde_json::Value, std::collections::HashMap<String, bool>), String> {
    let mut updated = baseline.clone();
    let mut enabled = std::collections::HashMap::new();

    let fields = flatten_defaults(baseline);
    for field in fields {
        enabled.insert(field.path, false);
    }
    for (key, _value) in form {
        if let Some(path) = key.strip_prefix("default_enabled.") {
            enabled.insert(path.to_string(), true);
        }
    }

    for (key, value) in form {
        if let Some(path) = key.strip_prefix("default_value.") {
            let type_key = format!("default_type.{path}");
            let kind = form.get(&type_key).map(String::as_str).unwrap_or("string");
            if value.trim().is_empty() {
                continue;
            }
            let parsed = parse_value_by_kind(kind, value)?;
            set_json_path(&mut updated, path, parsed)?;
        }
    }

    Ok((updated, enabled))
}

pub fn parse_value_by_kind(kind: &str, value: &str) -> Result<serde_json::Value, String> {
    match kind {
        "string" => Ok(serde_json::Value::String(value.to_string())),
        "number" => parse_number_value(value).map(serde_json::Value::Number),
        "bool" => match value.trim() {
            "true" => Ok(serde_json::Value::Bool(true)),
            "false" => Ok(serde_json::Value::Bool(false)),
            _ => Err(format!("invalid bool for {value}")),
        },
        "array" => serde_json::from_str(value)
            .map_err(|err| format!("invalid array json: {err}")),
        "null" => Ok(serde_json::Value::Null),
        _ => Ok(serde_json::Value::String(value.to_string())),
    }
}

fn parse_number_value(value: &str) -> Result<serde_json::Number, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("empty number".to_string());
    }
    if trimmed.contains('.') {
        let parsed = trimmed
            .parse::<f64>()
            .map_err(|_| format!("invalid number for {value}"))?;
        serde_json::Number::from_f64(parsed)
            .ok_or_else(|| format!("invalid number for {value}"))
    } else {
        let parsed = trimmed
            .parse::<i64>()
            .map_err(|_| format!("invalid number for {value}"))?;
        Ok(serde_json::Number::from(parsed))
    }
}

pub fn set_json_path(
    target: &mut serde_json::Value,
    path: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = target;
    for (idx, part) in parts.iter().enumerate() {
        let is_last = idx == parts.len() - 1;
        if is_last {
            if let Some(obj) = current.as_object_mut() {
                obj.insert(part.to_string(), value);
                return Ok(());
            } else {
                return Err(format!("invalid path: {path}"));
            }
        } else {
            current = current
                .get_mut(*part)
                .ok_or_else(|| format!("missing path segment: {part}"))?;
        }
    }
    Ok(())
}
