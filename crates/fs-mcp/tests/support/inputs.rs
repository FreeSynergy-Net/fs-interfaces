//! Arguments made from an `inputSchema`: a valid base, and one variant per place and per interesting value (the limits, zero,
//! empty, the longest text, every enumerant, every member of a union); and arguments that are wrong.
//! Test helpers; no pattern of their own.
#![allow(dead_code)]

use fs_mcp::serve::schema::resolve;
use serde_json::{Map, Value, json};

/// The id of a planted object (the canary store holds one).
pub fn planted_id() -> Value {
    json!({"hi": "1", "lo": "2"})
}

fn is_uuid(schema: &Value) -> bool {
    schema
        .get("$ref")
        .and_then(Value::as_str)
        .is_some_and(|r| r.ends_with("Uuid"))
}

/// One valid value for the schema (the first member of a union, the first enumerant, the lower limit).
fn base(root: &Value, schema: &Value, depth: u32) -> Value {
    if is_uuid(schema) {
        return planted_id();
    }
    let s = resolve(root, schema);
    if let Some(alts) = s.get("anyOf").and_then(Value::as_array) {
        return base(root, &alts[0], depth);
    }
    if let Some(e) = s.get("enum").and_then(Value::as_array) {
        return e[0].clone();
    }
    match s.get("type").and_then(Value::as_str) {
        Some("object") => {
            let props = s.get("properties").and_then(Value::as_object);
            let mut want: Vec<&str> = s
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            if let Some(alt) = s
                .get("oneOf")
                .and_then(Value::as_array)
                .and_then(|a| a.first())
            {
                want.extend(
                    alt["required"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str),
                );
            }
            Value::Object(
                want.into_iter()
                    .filter_map(|k| Some((k.to_owned(), base(root, &props?[k], depth + 1))))
                    .collect(),
            )
        }
        Some("array") if depth < 5 => json!([base(root, &s["items"], depth + 1)]),
        Some("array") => json!([]),
        Some("string") if s.get("x-fs-int").is_some() => json!("0"),
        Some("string") if s.get("contentEncoding").is_some() => json!("AAAA"),
        Some("string") => json!("x"),
        Some("integer") => s.get("minimum").cloned().unwrap_or(json!(0)),
        Some("number") => json!(0),
        Some("boolean") => json!(false),
        _ => Value::Null,
    }
}

/// Every base: one for each member of a union at the top.
fn bases(root: &Value) -> Vec<Value> {
    let mut out = vec![base(root, root, 0)];
    if let Some(alts) = root.get("oneOf").and_then(Value::as_array) {
        for alt in alts {
            let Some(Value::Object(mut m)) = out.first().cloned() else {
                continue;
            };
            let props = root.get("properties").and_then(Value::as_object);
            for k in alt["required"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                if let Some(ps) = props.and_then(|p| p.get(k)) {
                    m.insert(k.to_owned(), base(root, ps, 1));
                }
            }
            // only this member of the union
            if let Some(members) = props {
                let keep: Vec<&str> = alt["required"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                for other in root["oneOf"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|a| a["required"][0].as_str())
                {
                    if !keep.contains(&other) && members.contains_key(other) {
                        m.remove(other);
                    }
                }
            }
            out.push(Value::Object(m));
        }
    }
    out
}

/// The interesting values for a place with this schema.
fn alternatives(root: &Value, schema: &Value) -> Vec<Value> {
    if is_uuid(schema) {
        return vec![
            json!({"hi": "0", "lo": "0"}),
            json!({"hi": "18446744073709551615", "lo": "18446744073709551615"}),
        ];
    }
    let s = resolve(root, schema);
    if let Some(e) = s.get("enum").and_then(Value::as_array) {
        return e.clone();
    }
    match s.get("type").and_then(Value::as_str) {
        Some("string") if s.get("x-fs-int").is_some() => {
            let r = &s["x-fs-int"];
            vec![r["minimum"].clone(), r["maximum"].clone(), json!("0")]
        }
        Some("string") if s.get("contentEncoding").is_some() => {
            vec![json!(""), json!("AAAA"), json!("A".repeat(4096))]
        }
        Some("string") => vec![
            json!(""),
            json!("x"),
            json!("a".repeat(4096)),
            json!("../../etc/passwd"),
            json!("%00\u{0}\u{202e}"),
        ],
        Some("integer") => vec![
            s.get("minimum").cloned().unwrap_or(json!(0)),
            s.get("maximum").cloned().unwrap_or(json!(2_147_483_647)),
            json!(0),
        ],
        Some("number") => vec![json!(0), json!(1.0e300), json!("NaN"), json!("Infinity")],
        Some("boolean") => vec![json!(true), json!(false)],
        Some("array") => vec![
            json!([]),
            json!([base(root, &s["items"], 3), base(root, &s["items"], 3)]),
        ],
        _ => Vec::new(),
    }
}

/// The places of a base value that are leaves (or lists), as JSON pointers with their schemas.
fn places<'a>(
    root: &'a Value,
    schema: &'a Value,
    value: &Value,
    ptr: &str,
    out: &mut Vec<(String, &'a Value)>,
) {
    let s = resolve(root, schema);
    match (value, s.get("type").and_then(Value::as_str)) {
        (Value::Object(m), Some("object")) if !is_uuid(schema) => {
            for (k, v) in m {
                if let Some(ps) = s.get("properties").and_then(|p| p.get(k)) {
                    places(root, ps, v, &format!("{ptr}/{k}"), out);
                }
            }
        }
        _ => out.push((ptr.to_owned(), schema)),
    }
}

/// Valid arguments: the bases, and every base with one place changed to each interesting value.
pub fn variants(schema_text: &str) -> Vec<Value> {
    let root: Value = serde_json::from_str(schema_text).expect("schema");
    let mut out = Vec::new();
    for b in bases(&root) {
        out.push(b.clone());
        let mut at = Vec::new();
        places(&root, &root, &b, "", &mut at);
        for (ptr, ps) in at {
            if ptr.is_empty() {
                continue;
            }
            for alt in alternatives(&root, ps) {
                let mut v = b.clone();
                if let Some(slot) = v.pointer_mut(&ptr) {
                    *slot = alt;
                    if !out.contains(&v) {
                        out.push(v);
                    }
                }
            }
        }
    }
    out.truncate(200);
    out
}

/// Arguments that are wrong: a place of the wrong type, a missing one, an extra one, and arguments that are no object.
pub fn wrong(schema_text: &str) -> Vec<Value> {
    let root: Value = serde_json::from_str(schema_text).expect("schema");
    let b = base(&root, &root, 0);
    let mut out = vec![json!([]), json!("x"), json!(1), Value::Null];
    let Value::Object(m) = b else { return out };
    let mut extra = m.clone();
    extra.insert("zzUnknown".to_owned(), json!(1));
    out.push(Value::Object(extra));
    for k in m.keys() {
        let mut less: Map<String, Value> = m.clone();
        less.remove(k);
        out.push(Value::Object(less));
        for bad in [
            json!(true),
            json!(1),
            json!("x"),
            json!([]),
            json!({}),
            Value::Null,
            json!(-1),
            json!(1.5),
        ] {
            let mut w = m.clone();
            w.insert(k.clone(), bad);
            out.push(Value::Object(w));
        }
    }
    out
}
