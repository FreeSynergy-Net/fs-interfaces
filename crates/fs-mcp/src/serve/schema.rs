//! Reading the JSON Schemas of the tool table at run time: references into `$defs`, the properties of an object.
//!
//! Pattern: none (small pure helpers over `serde_json::Value`). The schemas of the generator only use local
//! `#/$defs/...` references, never anything over the network.

use serde_json::Value;

/// The schema behind a `$ref` (`#/$defs/common.Uuid`) in `root`; the schema itself if it has none.
#[must_use]
pub fn resolve<'a>(root: &'a Value, schema: &'a Value) -> &'a Value {
    match schema.get("$ref").and_then(Value::as_str) {
        Some(r) => r
            .strip_prefix("#/$defs/")
            .and_then(|k| root.get("$defs")?.get(k))
            .unwrap_or(schema),
        None => schema,
    }
}

/// The schema of the property `name` of an object schema, references resolved.
#[must_use]
pub fn property<'a>(root: &'a Value, schema: &'a Value, name: &str) -> Option<&'a Value> {
    let s = resolve(root, schema);
    s.get("properties")?.get(name).map(|p| resolve(root, p))
}

/// The schema of the elements of an array schema.
#[must_use]
pub fn items<'a>(root: &'a Value, schema: &'a Value) -> Option<&'a Value> {
    resolve(root, schema).get("items").map(|p| resolve(root, p))
}

/// Is this a 64-bit integer, which the capnp-JSON codec writes as a string (`x-fs-int`)?
#[must_use]
pub fn is_wide_int(schema: &Value) -> bool {
    schema.get("x-fs-int").is_some()
}
