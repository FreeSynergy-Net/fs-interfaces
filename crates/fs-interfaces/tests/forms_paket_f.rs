//! Paket F: `bars-in-time@1.1.0`, `checklist@1.1.0`, the third kind of line (`= interval ...`, `= parent ...`, `forms::Relation`) and the reader that
//! must not go silently empty (the catalog falls back to an EMPTY list when the built-in file does not parse, `catalog.rs`).

use fs_interfaces::forms::{self, Art, ErrorKind, Relation};

fn writes(form: &forms::Form, command: &str) -> Vec<(String, bool)> {
    form.channel(command)
        .unwrap()
        .writes
        .as_ref()
        .unwrap()
        .iter()
        .map(|w| (w.channel.clone(), w.required))
        .collect()
}

fn w(name: &str, required: bool) -> (String, bool) {
    (name.to_owned(), required)
}

/// The real file parses (no stub), and the catalog is the parsed list, not the silent empty fallback.
#[test]
fn the_built_in_file_parses_and_the_catalog_is_not_empty() {
    let parsed = forms::parse(forms::SOURCE).expect("schema/forms.txt parses");
    assert!(parsed.len() > 10);
    assert_eq!(forms::catalog().len(), parsed.len());
    assert!(forms::find_key("bars-in-time@1.1.0").is_some());
    assert!(forms::find_key("checklist@1.1.0").is_some());
}

#[test]
fn bars_in_time_1_1_0_keeps_1_0_0_and_adds_the_commands() {
    let old = forms::find_key("bars-in-time@1.0.0").unwrap();
    let new = forms::find_key("bars-in-time@1.1.0").unwrap();
    for c in &old.channels {
        let n = new.channel(&c.name).unwrap();
        assert_eq!(
            (&c.arts, c.required, c.count),
            (&n.arts, n.required, n.count),
            "{}",
            c.name
        );
    }
    assert_eq!(new.channels.len(), old.channels.len() + 4);
    assert_eq!(new.channel("color").unwrap().arts, [Art::Text]);
    let required: Vec<&str> = new.required().map(|c| c.name.as_str()).collect();
    assert_eq!(required, ["items", "start", "end", "label"]);
    assert_eq!(
        writes(new, "move"),
        [w("start", true), w("end", true), w("lane", false)]
    );
    assert_eq!(
        writes(new, "create"),
        [
            w("label", true),
            w("start", true),
            w("end", true),
            w("all-day", false),
            w("lane", false)
        ]
    );
    assert_eq!(writes(new, "remove").len(), 0);
    assert_eq!(writes(new, "open").len(), 0);
    assert_eq!(old.relations.len(), 0);
    assert_eq!(
        new.relations,
        [Relation::Interval {
            start: "start".into(),
            end: "end".into(),
            all_day: Some("all-day".into())
        }]
    );
}

#[test]
fn checklist_1_1_0_keeps_1_0_0_and_adds_the_row_channels() {
    let old = forms::find_key("checklist@1.0.0").unwrap();
    let new = forms::find_key("checklist@1.1.0").unwrap();
    for c in &old.channels {
        let n = new.channel(&c.name).unwrap();
        assert_eq!(
            (&c.arts, c.required, c.count),
            (&n.arts, n.required, n.count)
        );
    }
    let added = ["sub", "facts", "group", "toggle", "create", "open"];
    assert_eq!(new.channels.len(), old.channels.len() + added.len());
    for name in added {
        assert!(!new.channel(name).unwrap().required, "{name} is optional");
        assert!(old.channel(name).is_none(), "{name} is new");
    }
    let required: Vec<&str> = new.required().map(|c| c.name.as_str()).collect();
    assert_eq!(required, ["items", "label", "done"]);
    assert_eq!(new.channel("facts").unwrap().count.max, Some(4));
    assert_eq!(writes(new, "toggle"), [w("done", true)]);
    assert_eq!(writes(new, "create"), [w("label", true), w("group", false)]);
    assert_eq!(writes(new, "open").len(), 0);
    assert_eq!(new.relations.len(), 0);
}

const BASE: &str = "f@1.0.0 | F | planned | d\n> items | records | ! | 1\n> a | time | ! | 1\n> b | time | ! | 1\n> day | flag | - | 0..1\n> up | reference | - | 0..1\n> many | time | - | 0..n\n";

fn err(relations: &str) -> (usize, ErrorKind) {
    let e = forms::parse(&format!("{BASE}{relations}")).unwrap_err();
    (e.line, e.kind)
}

#[test]
fn a_relation_line_is_read_with_and_without_the_day_flag() {
    let f = &forms::parse(&format!("{BASE}= interval a b\n= parent up\n")).unwrap()[0];
    assert_eq!(
        f.relations,
        [
            Relation::Interval {
                start: "a".into(),
                end: "b".into(),
                all_day: None
            },
            Relation::Parent {
                channel: "up".into()
            }
        ]
    );
}

#[test]
fn a_bad_relation_is_refused_with_its_line() {
    assert_eq!(
        err("= interval a b day extra\n"),
        (8, ErrorKind::RelationShape)
    );
    assert_eq!(err("= parent\n"), (8, ErrorKind::RelationShape));
    assert_eq!(err("= nested a b\n"), (8, ErrorKind::RelationKind));
    assert_eq!(err("=\n"), (8, ErrorKind::RelationKind));
    assert_eq!(
        err("= interval a nope\n"),
        (8, ErrorKind::RelationUnknownChannel)
    );
    assert_eq!(err("= interval a day\n"), (8, ErrorKind::RelationArt));
    assert_eq!(err("= interval a b a\n"), (8, ErrorKind::RelationDuplicate));
    assert_eq!(err("= interval a a\n"), (8, ErrorKind::RelationDuplicate));
    assert_eq!(err("= interval a many\n"), (8, ErrorKind::RelationArt));
    assert_eq!(err("= parent a\n"), (8, ErrorKind::RelationArt));
    assert_eq!(
        err("= interval a b\n= interval b a\n"),
        (9, ErrorKind::RelationDuplicate)
    );
}

#[test]
fn a_relation_before_any_form_is_refused() {
    let e = forms::parse("= interval a b\n").unwrap_err();
    assert_eq!((e.line, e.kind), (1, ErrorKind::RelationWithoutForm));
}

#[test]
fn the_fifth_column_stays_refused_on_data_channels() {
    let text = "f@1.0.0 | F | planned | d\n> items | records | ! | 1\n> a | time | - | 0..1 | x\n";
    let e = forms::parse(text).unwrap_err();
    assert_eq!((e.line, e.kind), (3, ErrorKind::WritesOnNonCommand));
}

/// A mutant of the built-in file (the interval names a channel that is not there) must not parse: it would turn the whole catalog empty.
#[test]
fn a_broken_relation_in_the_real_file_would_be_caught() {
    let broken = forms::SOURCE.replace(
        "= interval start end all-day",
        "= interval start finish all-day",
    );
    assert_ne!(broken, forms::SOURCE);
    let e = forms::parse(&broken).unwrap_err();
    assert_eq!(e.kind, ErrorKind::RelationUnknownChannel);
}
