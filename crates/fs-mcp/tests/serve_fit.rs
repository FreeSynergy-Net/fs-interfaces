//! The check of arguments before a call (`serve::fit`) against the `jsonschema` crate, on every tool of the real table:
//! a valid sample and one broken copy per property must be judged alike by both. Pattern: none (tests).

mod support;

use fs_mcp::serve::fit::fits;
use fs_mcp::serve::schema::resolve;
use fs_mcp::tasks_mcp;
use jsonschema::Validator;
use serde_json::{Map, Value, json};

/// A value that fits `schema` (the subset the generator writes).
fn sample(root: &Value, schema: &Value, depth: u32) -> Value {
    let s = resolve(root, schema);
    if let Some(alts) = s.get("anyOf").and_then(Value::as_array) {
        return sample(root, &alts[0], depth);
    }
    if let Some(e) = s.get("enum").and_then(Value::as_array) {
        return e[0].clone();
    }
    match s.get("type").and_then(Value::as_str) {
        Some("object") => {
            let mut m = Map::new();
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
            for k in want {
                let ps = &props.expect("properties")[k];
                m.insert(k.to_owned(), sample(root, ps, depth + 1));
            }
            Value::Object(m)
        }
        Some("array") if depth < 6 => json!([sample(root, &s["items"], depth + 1)]),
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

fn agree(root: &Value, v: &Validator, args: &Value, expect: bool, what: &str) {
    assert_eq!(
        fits(root, root, args).is_ok(),
        expect,
        "fit: {what}: {args}"
    );
    assert_eq!(v.is_valid(args), expect, "jsonschema: {what}: {args}");
}

#[test]
fn the_check_and_the_jsonschema_crate_judge_every_tool_alike() {
    let mut checked = 0;
    for t in tasks_mcp::TOOLS {
        let root: Value = serde_json::from_str(t.input_schema).unwrap();
        let v = Validator::new(&root).expect("a schema");
        let ok = sample(&root, &root, 0);
        agree(&root, &v, &ok, true, t.name);
        let Value::Object(m) = &ok else {
            panic!("{}", t.name)
        };
        // an unknown argument
        let mut extra = m.clone();
        extra.insert("zzUnknown".to_owned(), json!(1));
        agree(
            &root,
            &v,
            &Value::Object(extra),
            false,
            &format!("{} extra", t.name),
        );
        for k in m.keys() {
            let mut less = m.clone();
            less.remove(k);
            // a union member that is the only one given is "required" by its alternative: both reject
            agree(
                &root,
                &v,
                &Value::Object(less),
                false,
                &format!("{} without {k}", t.name),
            );
            // the wrong type
            let mut wrong = m.clone();
            wrong.insert(k.clone(), json!(true));
            agree(
                &root,
                &v,
                &Value::Object(wrong),
                false,
                &format!("{} {k} as bool", t.name),
            );
            let mut wrong = m.clone();
            wrong.insert(k.clone(), json!([[]]));
            agree(
                &root,
                &v,
                &Value::Object(wrong),
                false,
                &format!("{} {k} as list", t.name),
            );
            checked += 1;
        }
        agree(&root, &v, &json!([]), false, "not an object");
    }
    assert!(checked >= 20, "{checked}");
}

#[test]
fn the_wide_integer_the_range_and_the_base64_are_checked_like_the_schema_says() {
    let root = json!({
    "type":"object","additionalProperties":false,"required":["a","b","c"],
    "properties":{
        "a":{"type":"string","pattern":"^[0-9]+$","x-fs-int":{"signed":false,"bits":64,"minimum":"0","maximum":"18446744073709551615"}},
        "b":{"type":"integer","minimum":0,"maximum":255},
        "c":{"type":"string","contentEncoding":"base64"}
    }});
    let ok =
        |a: Value, b: Value, c: Value| fits(&root, &root, &json!({"a": a, "b": b, "c": c})).is_ok();
    assert!(ok(json!("18446744073709551615"), json!(255), json!("Zm9v")));
    assert!(!ok(
        json!("18446744073709551616"),
        json!(255),
        json!("Zm9v")
    ));
    assert!(
        !ok(json!(1), json!(255), json!("Zm9v")),
        "a 64-bit integer as a number is refused"
    );
    assert!(!ok(json!("-1"), json!(0), json!("Zm9v")));
    assert!(!ok(json!("1"), json!(256), json!("Zm9v")));
    assert!(!ok(json!("1"), json!(-1), json!("Zm9v")));
    assert!(!ok(json!("1"), json!(1.5), json!("Zm9v")));
    assert!(!ok(json!("1"), json!(1), json!("Zm9")));
}

#[test]
fn a_union_needs_exactly_one_member() {
    let root = json!({"type":"object","additionalProperties":false,"properties":{"x":{"type":"boolean"},"y":{"type":"boolean"}},"oneOf":[{"required":["x"]},{"required":["y"]}]});
    assert!(fits(&root, &root, &json!({"x": true})).is_ok());
    assert!(fits(&root, &root, &json!({"y": true})).is_ok());
    assert!(fits(&root, &root, &json!({})).is_err());
    assert!(fits(&root, &root, &json!({"x": true, "y": true})).is_err());
}

#[test]
fn a_float_is_a_number_or_one_of_the_three_strings_and_an_enum_is_one_of_its_names() {
    let root = json!({"type":"object","additionalProperties":false,"required":["f","e"],"properties":{
        "f":{"anyOf":[{"type":"number"},{"type":"string","enum":["Infinity","-Infinity","NaN"]}]},
        "e":{"type":"string","enum":["a","b"]}}});
    let v = Validator::new(&root).unwrap();
    for (f, e, ok) in [
        (json!(1.5), json!("a"), true),
        (json!(0), json!("b"), true),
        (json!("NaN"), json!("a"), true),
        (json!("Infinity"), json!("a"), true),
        (json!("-Infinity"), json!("a"), true),
        (json!("x"), json!("a"), false),
        (json!(true), json!("a"), false),
        (json!(null), json!("a"), false),
        (json!(1.5), json!("c"), false),
        (json!(1.5), json!("A"), false),
        (json!(1.5), json!(0), false),
    ] {
        agree(&root, &v, &json!({"f": f, "e": e}), ok, &format!("{f} {e}"));
    }
}

#[test]
fn the_check_fails_closed_on_what_it_cannot_read() {
    // a reference that does not resolve
    let root = json!({"type":"object","properties":{"a":{"$ref":"#/$defs/Nowhere"}}});
    assert!(fits(&root, &root, &json!({"a": 1})).is_err());
    let root = json!({"type":"object","properties":{"a":{"$ref":"http://example.org/x"}}});
    assert!(fits(&root, &root, &json!({"a": 1})).is_err());
    let root = json!({"type":"object","properties":{"a":{"$ref":5}}});
    assert!(fits(&root, &root, &json!({"a": 1})).is_err());
    // a reference that does not resolve is an error even if the schema next to it names a type
    let root =
        json!({"type":"object","properties":{"a":{"$ref":"#/$defs/Nowhere","type":"string"}}});
    assert!(fits(&root, &root, &json!({"a": "x"})).is_err());
    // a schema that names no type
    let root = json!({"type":"object","properties":{"a":{"description":"anything?"}}});
    assert!(fits(&root, &root, &json!({"a": 1})).is_err());
    // a pattern it does not know
    let root = json!({"type":"object","properties":{"a":{"type":"string","pattern":"^[a-z]+$"}}});
    assert!(fits(&root, &root, &json!({"a": "abc"})).is_err());
    // an enum alone is a schema; a known pattern is checked
    let root = json!({"type":"object","properties":{"a":{"enum":["x"]}}});
    assert!(fits(&root, &root, &json!({"a": "x"})).is_ok());
    let wide = |p: &str| json!({"type":"object","properties":{"a":{"type":"string","pattern":p,"x-fs-int":{"minimum":"-5","maximum":"5"}}}});
    let unsigned = wide("^[0-9]+$");
    let signed = wide("^-?[0-9]+$");
    for (root, text, ok) in [
        (&unsigned, "5", true),
        (&unsigned, "+5", false),
        (&unsigned, "-1", false),
        (&unsigned, "", false),
        (&unsigned, "5 ", false),
        (&unsigned, "0x5", false),
        (&signed, "-5", true),
        (&signed, "+5", false),
        (&signed, "-", false),
        (&signed, "--5", false),
        (&signed, "", false),
        (&signed, "6", false),
    ] {
        assert_eq!(
            fits(root, root, &json!({"a": text})).is_ok(),
            ok,
            "{text:?}"
        );
    }
}

#[test]
fn base64_must_be_canonical_in_the_arguments() {
    let root =
        json!({"type":"object","properties":{"d":{"type":"string","contentEncoding":"base64"}}});
    for (text, ok) in [
        ("Zm9v", true),
        ("Zg==", true),
        ("", true),
        ("A===", false),
        ("====", false),
        ("Zg=", false),
        ("Zh==", false),
        ("Zm9", false),
        ("Zm9v\n", false),
    ] {
        assert_eq!(
            fits(&root, &root, &json!({"d": text})).is_ok(),
            ok,
            "{text:?}"
        );
    }
}
