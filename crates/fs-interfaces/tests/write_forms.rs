//! SW-1 (Schreibweg, ENTSCHEID-schnitt section 3), part 1: the column `writes` of `forms.txt` and the form `cards-in-columns@1.1.0`.
//! Everything is appended: the old form `cards-in-columns@1.0.0` stays as it was. Part 2 (capnp, capabilities, claim): `write_path.rs`.

use fs_interfaces::forms::{self, ErrorKind, parse};

const HEAD: &str = "a@1.0.0 | A | planned | text\n> items | records | ! | 1\n> title | text | ! | 1\n> facts | text|number | - | 0..4\n";

fn with(channels: &str) -> String {
    format!("{HEAD}{channels}")
}

fn err(text: &str) -> (usize, ErrorKind) {
    let e = parse(text).unwrap_err();
    (e.line, e.kind)
}

// ---- forms.txt: the column `writes` ----

#[test]
fn writes_are_read_with_the_required_mark_and_empty_means_nothing() {
    let all = parse(&with(
        "> go | command | - | 0..1 | title! other\n> other | text | - | 0..1\n> del | command | - | 0..1 |\n> old | command | - | 0..1\n",
    ))
    .unwrap();
    let f = &all[0];
    let go = f.channel("go").unwrap().writes.as_ref().unwrap();
    assert_eq!(go.len(), 2);
    assert_eq!((go[0].channel.as_str(), go[0].required), ("title", true));
    assert_eq!((go[1].channel.as_str(), go[1].required), ("other", false));
    assert_eq!(f.channel("del").unwrap().writes, Some(vec![]));
    assert_eq!(
        f.channel("old").unwrap().writes,
        None,
        "a line without the 5th field stays readable"
    );
}

#[test]
fn writes_on_an_unknown_channel_is_an_error_with_the_line_of_the_command() {
    let text = with("> go | command | - | 0..1 | title nope\n");
    assert_eq!(err(&text), (5, ErrorKind::WritesUnknownChannel));
}

#[test]
fn writes_on_a_channel_with_more_than_one_field_is_an_error() {
    let text = with("> go | command | - | 0..1 | facts\n");
    assert_eq!(err(&text), (5, ErrorKind::WritesManyChannel));
    let text = with("> many | text | - | 0..n\n> go | command | - | 0..1 | many\n");
    assert_eq!(err(&text), (6, ErrorKind::WritesManyChannel));
}

#[test]
fn writes_may_name_a_channel_declared_later_but_not_a_command() {
    let ok = with("> go | command | - | 0..1 | late\n> late | text | - | 0..1\n");
    assert!(parse(&ok).is_ok());
    let text = with("> go | command | - | 0..1 | other\n> other | command | - | 0..1\n");
    assert_eq!(err(&text), (5, ErrorKind::WritesCommandChannel));
}

#[test]
fn writes_only_on_a_command_and_well_formed() {
    assert_eq!(
        err(&with("> x | text | - | 0..1 | title\n")),
        (5, ErrorKind::WritesOnNonCommand)
    );
    assert_eq!(
        err(&with("> go | command | - | 0..1 | Title\n")),
        (5, ErrorKind::WritesName)
    );
    assert_eq!(
        err(&with("> go | command | - | 0..1 | title title!\n")),
        (5, ErrorKind::WritesDuplicate)
    );
    assert_eq!(
        err(&with("> go | command | - | 0..1 | title | extra\n")),
        (5, ErrorKind::Shape)
    );
}

// ---- cards-in-columns@1.1.0 ----

#[test]
fn cards_in_columns_1_1_0_is_appended_and_the_old_slot_is_unchanged() {
    let old = forms::find_key("cards-in-columns@1.0.0").unwrap();
    let names: Vec<_> = old.channels.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        ["cards", "column", "title", "facts", "lane", "move"],
        "1.0.0 stays as it was"
    );
    assert!(old.channels.iter().all(|c| c.writes.is_none()));

    let f = forms::find_key("cards-in-columns@1.1.0").unwrap();
    for n in &names {
        let (a, b) = (old.channel(n).unwrap(), f.channel(n).unwrap());
        assert_eq!(
            (&a.arts, a.required, a.count),
            (&b.arts, b.required, b.count),
            "{n}"
        );
    }
    for optional in ["order", "column-label", "progress", "create", "remove"] {
        let c = f.channel(optional).unwrap();
        assert!(!c.required, "{optional} is optional");
    }
    let required: Vec<_> = f.required().map(|c| c.name.as_str()).collect();
    assert_eq!(required, ["cards", "column", "title"]);
}

#[test]
fn progress_is_its_own_channel_and_facts_keeps_its_limit() {
    let f = forms::find_key("cards-in-columns@1.1.0").unwrap();
    let progress = f.channel("progress").unwrap();
    assert_eq!(progress.arts, [forms::Art::Number]);
    assert_eq!(progress.count.max, Some(1));
    assert_eq!(f.channel("facts").unwrap().count.max, Some(4));
    let label = f.channel("column-label").unwrap();
    assert_eq!(label.arts, [forms::Art::Text]);
    assert_eq!(label.count.max, Some(1));
}

#[test]
fn the_commands_of_1_1_0_write_what_the_schnitt_says() {
    let f = forms::find_key("cards-in-columns@1.1.0").unwrap();
    let w = |c: &str| -> Vec<(String, bool)> {
        f.channel(c)
            .unwrap()
            .writes
            .as_ref()
            .unwrap()
            .iter()
            .map(|w| (w.channel.clone(), w.required))
            .collect()
    };
    let s = |n: &str, r| (n.to_owned(), r);
    assert_eq!(
        w("move"),
        [s("column", true), s("order", false), s("lane", false)]
    );
    assert_eq!(
        w("create"),
        [s("title", true), s("column", true), s("lane", false)]
    );
    assert_eq!(w("remove"), Vec::<(String, bool)>::new());
    // the move target is the ID: the label is never written
    assert!(!w("move").iter().any(|(n, _)| n == "column-label"));
}

#[test]
fn every_writes_entry_of_the_builtin_file_is_a_data_channel_with_room_for_one() {
    for f in forms::catalog() {
        for c in &f.channels {
            for w in c.writes.iter().flatten() {
                let t = f.channel(&w.channel).unwrap();
                assert_ne!(t.arts, [forms::Art::Command], "{}.{}", f.key(), c.name);
                assert_eq!(t.count.max, Some(1), "{}.{}", f.key(), c.name);
            }
        }
    }
}
