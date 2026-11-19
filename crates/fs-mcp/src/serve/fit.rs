//! Do the arguments fit the `inputSchema` of the tool? Checked BEFORE anything runs, so that an argument that is
//! unknown, missing, mistyped or out of range never reaches the program (a delete with a stray field is not run).
//!
//! Pattern: Specification — a pure predicate over a JSON value and the subset of JSON Schema 2020-12 that the generator
//! writes (`type`, `properties`, `required`, `additionalProperties`, `items`, `enum`, `anyOf`, `oneOf` as alternatives of
//! `required`, `minimum`/`maximum`, `x-fs-int`, `contentEncoding`, local `$ref`). A schema keyword outside that subset is
//! not checked here; the tests compare this check with the `jsonschema` crate on every tool of the real table.

use serde_json::Value;

use super::{base64, schema};

/// Why the arguments do not fit, naming the path (`filter.due-from`), or `Ok`.
///
/// # Errors
/// The first violation found, as text for the tool result.
pub fn fits(root: &Value, schema: &Value, value: &Value) -> Result<(), String> {
    check(root, schema, value, "")
}

fn at(path: &str) -> String {
    if path.is_empty() {
        "the arguments".to_owned()
    } else {
        format!("`{path}`")
    }
}

fn join(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_owned()
    } else {
        format!("{path}.{name}")
    }
}

fn check(root: &Value, schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    let s = schema::resolve(root, schema);
    if let Some(alts) = s.get("anyOf").and_then(Value::as_array) {
        return if alts.iter().any(|a| check(root, a, value, path).is_ok()) {
            Ok(())
        } else {
            Err(format!("{} matches none of the allowed forms", at(path)))
        };
    }
    if let Some(names) = s.get("enum").and_then(Value::as_array)
        && !names.contains(value)
    {
        return Err(format!("{} is not one of the allowed values", at(path)));
    }
    match s.get("type").and_then(Value::as_str) {
        Some("object") => object(root, s, value, path),
        Some("array") => array(root, s, value, path),
        Some("string") => string(s, value, path),
        Some("integer") => integer(s, value, path),
        Some("number") if value.is_number() => Ok(()),
        Some("boolean") if value.is_boolean() => Ok(()),
        Some("null") if value.is_null() => Ok(()),
        Some(t) => Err(format!("{} must be of type {t}", at(path))),
        None => Ok(()),
    }
}

fn object(root: &Value, s: &Value, value: &Value, path: &str) -> Result<(), String> {
    let Some(m) = value.as_object() else {
        return Err(format!("{} must be an object", at(path)));
    };
    let props = s.get("properties").and_then(Value::as_object);
    let closed = s.get("additionalProperties") == Some(&Value::Bool(false));
    for (k, v) in m {
        match props.and_then(|p| p.get(k)) {
            Some(ps) => check(root, ps, v, &join(path, k))?,
            None if closed => {
                return Err(format!("{} is not a known argument", at(&join(path, k))));
            }
            None => {}
        }
    }
    for r in s
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(name) = r.as_str()
            && !m.contains_key(name)
        {
            return Err(format!("{} is required", at(&join(path, name))));
        }
    }
    if let Some(alts) = s.get("oneOf").and_then(Value::as_array) {
        let met = alts
            .iter()
            .filter(|a| {
                a.get("required")
                    .and_then(Value::as_array)
                    .is_some_and(|r| {
                        r.iter()
                            .all(|n| n.as_str().is_some_and(|n| m.contains_key(n)))
                    })
            })
            .count();
        if met != 1 {
            return Err(format!(
                "{} must name exactly one member of its union",
                at(path)
            ));
        }
    }
    Ok(())
}

fn array(root: &Value, s: &Value, value: &Value, path: &str) -> Result<(), String> {
    let Some(a) = value.as_array() else {
        return Err(format!("{} must be a list", at(path)));
    };
    if let Some(items) = s.get("items") {
        for (i, v) in a.iter().enumerate() {
            check(root, items, v, &join(path, &i.to_string()))?;
        }
    }
    Ok(())
}

fn string(s: &Value, value: &Value, path: &str) -> Result<(), String> {
    let Some(text) = value.as_str() else {
        return Err(format!("{} must be a string", at(path)));
    };
    if let Some(range) = s.get("x-fs-int") {
        // a 64-bit integer written as a string
        let n: i128 = text
            .parse()
            .map_err(|_| format!("{} must be an integer", at(path)))?;
        let bound = |k: &str| {
            range
                .get(k)
                .and_then(Value::as_str)
                .and_then(|b| b.parse::<i128>().ok())
        };
        if bound("minimum").is_some_and(|m| n < m) || bound("maximum").is_some_and(|m| n > m) {
            return Err(format!("{} is out of range", at(path)));
        }
    }
    if s.get("contentEncoding").and_then(Value::as_str) == Some("base64")
        && base64::decode(text).is_none()
    {
        return Err(format!("{} must be base64 text", at(path)));
    }
    Ok(())
}

fn integer(s: &Value, value: &Value, path: &str) -> Result<(), String> {
    let n = value
        .as_i64()
        .map(i128::from)
        .or_else(|| value.as_u64().map(i128::from))
        .ok_or_else(|| format!("{} must be an integer", at(path)))?;
    let bound = |k: &str| {
        s.get(k)
            .and_then(Value::as_i64)
            .map(i128::from)
            .or_else(|| s.get(k).and_then(Value::as_u64).map(i128::from))
    };
    if bound("minimum").is_some_and(|m| n < m) || bound("maximum").is_some_and(|m| n > m) {
        return Err(format!("{} is out of range", at(path)));
    }
    Ok(())
}
