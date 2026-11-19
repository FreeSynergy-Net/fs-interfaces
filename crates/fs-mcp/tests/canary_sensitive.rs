//! Sensitive data of a person (class `FSSENS`): it leaves the server only to a recipient that has a release NOW, and the recipient
//! is the holder of the key of the connection, never a label of the client. Pattern: none (tests).
//!
//! A diary stand-in (`support/diary.rs`) with tools whose results have sensitive places (`x-fs-sensitive`): a list of entries, a
//! struct, an export.
#![cfg(feature = "conformance")]

mod support;

use fs_mcp::canary::{Class, scan};
use fs_mcp::serve::{Connection, ConnectionKey, Server, ServerInfo, Started};
use futures::executor::LocalPool;
use serde_json::{Value, json};
use support::diary::{DiaryCalls, Everybody, Released, World, ask, catalog, server, table, world};
use support::serve::{meta, request};

const SENSITIVE_TOOLS: [&str; 3] = ["diary.read", "diary.mood", "diary.export"];

fn leaks(w: &World, text: &str) -> bool {
    !scan(text, w.table.all()).is_empty()
}
fn is_error(text: &str) -> bool {
    serde_json::from_str::<Value>(text).unwrap()["result"]["isError"] == json!(true)
}
fn key(holder: &str) -> ConnectionKey {
    ConnectionKey::verified(holder)
}

#[test]
fn a_without_a_key_nothing_leaves_in_any_answer_or_export_even_if_everybody_is_released() {
    let w = world();
    let s = server(&w, Connection::keyless(), Box::new(Everybody));
    for t in SENSITIVE_TOOLS {
        for client in [None, Some("ai:A"), Some("A")] {
            let a = ask(&s, t, client);
            assert!(!leaks(&w, &a), "{t}: {a}");
            assert!(is_error(&a), "{t} is not an error: {a}");
        }
    }
    // the default server (no `with_access`) is the same
    let default = Server::new(
        DiaryCalls {
            sens: w.sens.clone(),
            secret: w.secret.clone(),
        },
        table(),
        &catalog(),
        &Started,
        ServerInfo {
            name: "d".into(),
            version: "0".into(),
        },
    )
    .unwrap();
    for t in SENSITIVE_TOOLS {
        assert!(!leaks(&w, &ask(&default, t, None)), "{t}");
    }
    // what is not sensitive works
    let plain = ask(&s, "diary.plain", None);
    assert!(!is_error(&plain) && plain.contains("my diary"), "{plain}");
}

#[test]
fn the_error_names_no_place_and_no_value() {
    let w = world();
    let s = server(&w, Connection::keyless(), Box::new(Released::default()));
    let a = ask(&s, "diary.read", None);
    for word in ["line", "entries", "day", "monday", "my diary"] {
        assert!(!a.contains(word), "{word} in {a}");
    }
}

#[test]
fn b_a_release_for_a_reaches_a_and_nobody_else_and_a_label_decides_nothing() {
    let w = world();
    let released = Released::default();
    released.0.borrow_mut().insert("A".into());
    let a = server(&w, Connection::keyed(&key("A")), Box::new(released.clone()));
    let b = server(&w, Connection::keyed(&key("B")), Box::new(released.clone()));
    for t in SENSITIVE_TOOLS {
        let ra = ask(&a, t, Some("whatever"));
        assert!(
            !is_error(&ra) && ra.contains(&w.sens),
            "A gets nothing from {t}: {ra}"
        );
        // B: no release, also when it calls itself `ai:A`, `A` or anything
        for label in [None, Some("ai:A"), Some("A"), Some("ai:B")] {
            let rb = ask(&b, t, label);
            assert!(
                !leaks(&w, &rb) && is_error(&rb),
                "B gets {t} as {label:?}: {rb}"
            );
        }
    }
}

#[test]
fn c_a_revocation_stops_the_next_call() {
    let w = world();
    let released = Released::default();
    released.0.borrow_mut().insert("A".into());
    let a = server(&w, Connection::keyed(&key("A")), Box::new(released.clone()));
    for t in SENSITIVE_TOOLS {
        assert!(ask(&a, t, None).contains(&w.sens), "{t} before");
    }
    released.0.borrow_mut().clear();
    for t in SENSITIVE_TOOLS {
        let r = ask(&a, t, None);
        assert!(
            !leaks(&w, &r) && is_error(&r),
            "{t} after the revocation: {r}"
        );
    }
}

#[test]
fn d_the_export_follows_the_same_rule() {
    let w = world();
    let released = Released::default();
    let a = server(&w, Connection::keyed(&key("A")), Box::new(released.clone()));
    assert!(!leaks(&w, &ask(&a, "diary.export", None)));
    released.0.borrow_mut().insert("A".into());
    assert!(ask(&a, "diary.export", None).contains(&w.sens));
    released.0.borrow_mut().clear();
    assert!(!leaks(&w, &ask(&a, "diary.export", None)));
}

#[test]
fn e_a_secret_stays_secret_with_a_release_for_everything() {
    let w = world();
    let s = server(&w, Connection::keyed(&key("A")), Box::new(Everybody));
    // the screen keeps the tool with the secret-looking result out: it is not offered, and a call is no answer with the secret
    let a = ask(&s, "diary.token", None);
    assert!(!a.contains(&w.secret), "{a}");
    let list = LocalPool::new()
        .run_until(s.handle_line(&request(1, "tools/list", json!({"_meta": meta(None)}))))
        .unwrap();
    assert!(!list.contains("diary.token"));
    // and no answer of any tool holds the secret marker
    for t in SENSITIVE_TOOLS {
        let r = ask(&s, t, None);
        assert!(
            scan(&r, w.table.of_class(Class::Secret).as_slice()).is_empty(),
            "{r}"
        );
    }
}

#[test]
fn a_struct_that_is_marked_is_held_back_whole_and_a_list_of_marked_places_too() {
    let w = world();
    let s = server(&w, Connection::keyless(), Box::new(Released::default()));
    let r = ask(&s, "diary.mood", None);
    assert!(is_error(&r) && !r.contains("score"), "{r}");
}
