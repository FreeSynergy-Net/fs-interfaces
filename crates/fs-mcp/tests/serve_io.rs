//! `JsonInput` (named values from JSON) and `JsonOutput` (a result as JSON in the form of the schema). Pattern: none (tests).

use capnp_compiler::cligen::rt::{Input, Output, Value as V};
use fs_mcp::serve::input::JsonInput;
use fs_mcp::serve::output::JsonOutput;
use serde_json::{Value, json};

fn input(v: Value) -> JsonInput {
    JsonInput::new(v)
}

#[test]
fn paths_are_kebab_for_the_command_code_and_contract_names_in_the_json() {
    let mut i = input(
        json!({"otherUid": "u", "filter": {"dueFrom": "2026"}, "ids": ["a", "b"], "n": 5, "f": 1.5, "ok": true, "none": null}),
    );
    assert_eq!(i.text("other-uid").unwrap().as_deref(), Some("u"));
    assert_eq!(i.text("filter.due-from").unwrap().as_deref(), Some("2026"));
    assert_eq!(i.count("ids").unwrap(), Some(2));
    assert_eq!(i.text("ids.1").unwrap().as_deref(), Some("b"));
    assert_eq!(i.int("n").unwrap(), Some(5));
    assert_eq!(i.float("f").unwrap(), Some(1.5));
    assert_eq!(i.boolean("ok").unwrap(), Some(true));
    assert!(
        i.has("filter")
            && i.has("filter.due-from")
            && !i.has("none")
            && !i.has("missing")
            && !i.has("ids.2")
    );
    assert_eq!(i.text("missing").unwrap(), None);
}

#[test]
fn a_wrong_type_is_a_bad_value_of_that_path() {
    let mut i =
        input(json!({"t": 1, "n": "x", "b": "true", "l": "no", "e": 3, "d": "***", "f": "huge"}));
    assert_eq!(i.text("t").unwrap_err().path, "t");
    assert_eq!(i.int("n").unwrap_err().path, "n");
    assert!(
        i.boolean("b").is_err()
            && i.count("l").is_err()
            && i.data("d").is_err()
            && i.float("f").is_err()
    );
    assert!(i.enumerant("e", &["a"]).is_err());
}

#[test]
fn wide_integers_special_floats_enums_and_data() {
    let mut i = input(
        json!({"w": "18446744073709551615", "neg": "-9223372036854775808", "inf": "Infinity", "ninf": "-Infinity", "nan": "NaN", "e": "b", "bad": "z", "d": "Zm9v"}),
    );
    assert_eq!(i.int("w").unwrap(), Some(18_446_744_073_709_551_615));
    assert_eq!(i.int("neg").unwrap(), Some(i128::from(i64::MIN)));
    assert_eq!(i.float("inf").unwrap(), Some(f64::INFINITY));
    assert_eq!(i.float("ninf").unwrap(), Some(f64::NEG_INFINITY));
    assert!(i.float("nan").unwrap().unwrap().is_nan());
    assert_eq!(i.enumerant("e", &["a", "b"]).unwrap(), Some(1));
    // the JSON names the contract (`dependsOn`), the command code its kebab form
    let mut j = input(json!({"r": "dependsOn", "k": "depends-on"}));
    assert_eq!(j.enumerant("r", &["first", "depends-on"]).unwrap(), Some(1));
    // the kebab form maps too (it is its own kebab form); it is no name of the contract, and `fit` refuses it before
    assert_eq!(j.enumerant("k", &["first", "depends-on"]).unwrap(), Some(1));
    assert!(j.enumerant("r", &["first"]).is_err());
    assert!(i.enumerant("bad", &["a", "b"]).is_err());
    assert_eq!(i.data("d").unwrap().as_deref(), Some(&b"foo"[..]));
}

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "n": {"type": "integer"},
            "e": {"type": "string", "enum": ["open", "dependencyCancelled"]},
            "wide": {"type": "string", "x-fs-int": {"bits": 64}},
            "inner": {"$ref": "#/$defs/Inner"},
            "list": {"type": "array", "items": {"$ref": "#/$defs/Inner"}}
        },
        "$defs": {"Inner": {"type": "object", "properties": {"wide": {"type": "string", "x-fs-int": {"bits": 64}}, "small": {"type": "integer"}}}}
    })
}

#[test]
fn a_result_is_written_in_the_form_of_the_schema() {
    let s = schema();
    let mut o = JsonOutput::new(&s);
    o.begin_struct("", "");
    o.field("n", V::Int(-3));
    o.field("wide", V::Uint(u64::MAX));
    o.field("gone", V::Void);
    o.begin_struct("inner", "Inner");
    o.field("wide", V::Int(i64::MIN));
    o.field("small", V::Int(7));
    o.end();
    o.begin_list("list", "Inner");
    o.begin_struct("", "Inner");
    o.field("wide", V::Uint(1));
    o.field("small", V::Uint(2));
    o.end();
    o.end();
    o.field("t", V::Text("hi"));
    o.field("d", V::Data(b"foobar"));
    o.field("e", V::Enum("dependency-cancelled"));
    o.field("e2", V::Enum("not-in-schema"));
    o.field("nan", V::Float(f64::NAN));
    o.field("inf", V::Float(f64::INFINITY));
    o.field("ninf", V::Float(f64::NEG_INFINITY));
    o.field("fl", V::Float(0.5));
    o.field("b", V::Bool(true));
    o.end();
    assert_eq!(
        o.finish().unwrap(),
        json!({
            "n": -3, "wide": "18446744073709551615",
            "inner": {"wide": "-9223372036854775808", "small": 7},
            "list": [{"wide": "1", "small": 2}],
            "t": "hi", "d": "Zm9vYmFy", "e": "dependencyCancelled", "e2": "notInSchema", "nan": "NaN", "inf": "Infinity", "ninf": "-Infinity", "fl": 0.5, "b": true
        })
    );
}

#[test]
fn what_cannot_be_shown_makes_the_result_a_problem_never_a_number_or_a_hole() {
    let s = schema();
    for (name, v) in [
        ("e", V::UnknownEnum(9)),
        ("c", V::Capability("Items")),
        ("u", V::Unreadable("any pointer")),
    ] {
        let mut o = JsonOutput::new(&s);
        o.begin_struct("", "");
        o.field("n", V::Int(1));
        o.field(name, v);
        o.end();
        assert!(o.problem().is_some(), "{name}");
        let why = o.finish().unwrap_err();
        assert!(why.contains(&format!("`{name}`")), "{why}");
    }
    // an empty walk is no result
    assert!(JsonOutput::new(&s).finish().is_err());
}
