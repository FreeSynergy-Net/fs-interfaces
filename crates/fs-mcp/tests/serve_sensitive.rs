//! `JsonOutput` and the places marked `x-fs-sensitive`: where the mark sits, what it covers, and that the question is asked
//! per place and per call. Pattern: none (tests).

use std::cell::Cell;

use capnp_compiler::cligen::rt::{Output, Value as V};
use fs_mcp::serve::output::{JsonOutput, WITHHELD};
use fs_mcp::serve::{Access, Connection, ConnectionKey, DenyAll, Grants, Recipient};
use serde_json::{Value, json};

struct Counting<'conn>(&'conn Cell<usize>, bool);
impl Grants for Counting<'_> {
    fn allows(&self, _: &Recipient, _: &str, path: &str) -> bool {
        self.0.set(self.0.get() + 1);
        assert_ne!(path, "");
        self.1
    }
}

fn schema() -> Value {
    json!({"type":"object","properties":{
        "tags":{"type":"array","items":{"type":"string"},"x-fs-sensitive":true},
        "open":{"type":"string"},
        "inner":{"type":"object","properties":{"a":{"type":"string","x-fs-sensitive":true},"b":{"type":"string"}}}
    }})
}

fn write(out: &mut JsonOutput<'_>) {
    out.begin_struct("", "");
    out.begin_list("tags", "");
    out.field("", V::Text("t1"));
    out.field("", V::Text("t2"));
    out.end();
    out.field("open", V::Text("o"));
    out.begin_struct("inner", "");
    out.field("a", V::Text("A"));
    out.field("b", V::Text("B"));
    out.end();
    out.end();
}

#[test]
fn without_access_every_marked_place_is_held_back_and_the_error_names_nothing() {
    let sch = schema();
    let mut out = JsonOutput::new(&sch);
    write(&mut out);
    assert_eq!(out.finish().unwrap_err(), WITHHELD);
}

#[test]
fn a_connection_without_a_key_has_no_recipient_even_with_open_grants() {
    let sch = schema();
    let asked = Cell::new(0);
    let conn = Connection::keyless();
    let grants = Counting(&asked, true);
    let mut out = JsonOutput::new(&sch).with_access(Access {
        connection: &conn,
        grants: &grants,
        tool: "t",
    });
    write(&mut out);
    assert!(out.finish().is_err());
    assert_eq!(
        asked.get(),
        0,
        "the grants are not even asked without a recipient"
    );
}

#[test]
fn with_a_release_the_places_are_written_and_each_one_is_asked_each_time() {
    let sch = schema();
    let asked = Cell::new(0);
    let conn = Connection::keyed(&ConnectionKey::verified("A"));
    let grants = Counting(&asked, true);
    let mut out = JsonOutput::new(&sch).with_access(Access {
        connection: &conn,
        grants: &grants,
        tool: "t",
    });
    write(&mut out);
    let value = out.finish().unwrap();
    assert_eq!(value["tags"], json!(["t1", "t2"]));
    assert_eq!(value["inner"], json!({"a":"A","b":"B"}));
    // the list is asked when it opens and at both elements, `inner.a` once: 4 questions, none kept for a second call
    assert_eq!(asked.get(), 4);
    let mut again = JsonOutput::new(&sch).with_access(Access {
        connection: &conn,
        grants: &grants,
        tool: "t",
    });
    write(&mut again);
    assert_eq!(asked.get(), 8);
}

#[test]
fn a_denied_value_is_in_no_part_of_the_result() {
    let sch = schema();
    let conn = Connection::keyed(&ConnectionKey::verified("A"));
    let mut out = JsonOutput::new(&sch).with_access(Access {
        connection: &conn,
        grants: &DenyAll,
        tool: "t",
    });
    write(&mut out);
    assert!(out.problem().is_some());
    assert!(out.finish().is_err());
}
