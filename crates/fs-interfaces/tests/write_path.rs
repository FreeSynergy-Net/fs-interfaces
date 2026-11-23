//! SW-1 (Schreibweg, ENTSCHEID-schnitt section 3): the column `writes` of `forms.txt`, `cards-in-columns@1.1.0`, `RecordPage.may @5`,
//! `FormSource.invoke` with `result`, the capabilities `task.*` and `Items.claim @17` / `release @18` (no `by`). Everything is appended:
//! the old slots stay as they were.

use std::path::{Path, PathBuf};

use fs_interfaces::capabilities::{self, name};
use fs_interfaces::forms::{self, ErrorKind, parse};
use fs_interfaces::formsource_capnp as new;

#[path = "compat/formsource_v0_0_32_capnp.rs"]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
mod formsource_capnp;
#[allow(unused_imports)]
use fs_interfaces::common_capnp;

fn schema(name: &str) -> String {
    let dir: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schema");
    std::fs::read_to_string(dir.join(name)).unwrap()
}

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

// ---- formsource.capnp ----

#[test]
fn record_page_may_is_slot_5_and_an_old_reader_still_reads_the_page() {
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut page = msg.init_root::<new::record_page::Builder<'_>>();
        page.set_total(7);
        page.set_has_total(true);
        let mut may = page.init_may(2);
        may.set(0, "task.create");
        may.set(1, "card.move");
    }
    let bytes = capnp::serialize::write_message_to_words(&msg);
    let reader = capnp::serialize::read_message_from_flat_slice(
        &mut &bytes[..],
        capnp::message::ReaderOptions::new(),
    )
    .unwrap();
    let old = reader
        .get_root::<formsource_capnp::record_page::Reader<'_>>()
        .unwrap();
    assert_eq!(old.get_total(), 7);
    assert!(old.get_has_total());
    assert!(!old.get_has_estimate());
    let new_page = reader.get_root::<new::record_page::Reader<'_>>().unwrap();
    let may: Vec<_> = new_page
        .get_may()
        .unwrap()
        .iter()
        .map(|t| t.unwrap().to_string().unwrap())
        .collect();
    assert_eq!(may, ["task.create", "card.move"]);
    assert!(schema("formsource.capnp").contains("may            @5 :List(Text)"));
}

#[test]
fn an_old_page_has_an_empty_may() {
    let mut msg = capnp::message::Builder::new_default();
    msg.init_root::<formsource_capnp::record_page::Builder<'_>>()
        .set_total(1);
    let bytes = capnp::serialize::write_message_to_words(&msg);
    let reader = capnp::serialize::read_message_from_flat_slice(
        &mut &bytes[..],
        capnp::message::ReaderOptions::new(),
    )
    .unwrap();
    let page = reader.get_root::<new::record_page::Reader<'_>>().unwrap();
    assert_eq!(page.get_may().unwrap().len(), 0);
}

#[test]
fn invoke_result_is_appended_after_failed_and_problem() {
    let text = schema("formsource.capnp");
    assert!(text.contains("invoke @3 (command :Text, record :Text, args :List(Value)) -> (failed :Bool, problem :Common.Problem, result :WriteResult);"));
    assert!(text.contains("struct WriteResult { record @0 :Text; text @1 :Text; }"));
    // an old service answers without `result`: the new reader sees an empty one
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut r = msg.init_root::<new::write_result::Builder<'_>>();
        r.set_record("task-1");
        r.set_text("created");
    }
    let r = msg
        .get_root_as_reader::<new::write_result::Reader<'_>>()
        .unwrap();
    assert_eq!(r.get_record().unwrap().to_str().unwrap(), "task-1");
    assert_eq!(r.get_text().unwrap().to_str().unwrap(), "created");
}

// ---- capabilities.txt ----

#[test]
fn the_task_capabilities_are_on_the_list() {
    for n in [
        name::TASK_CREATE,
        name::TASK_DELETE,
        name::TASK_CLAIM,
        "task.edit",
        "task.assign",
        "task.read",
        "task.query",
        "task.import",
        "task.export",
        "task.subscribe",
        "relation.create",
        "label.assign",
    ] {
        assert!(capabilities::is_known(n), "{n}");
        assert!(!capabilities::is_planned(n), "{n} is built");
    }
}

#[test]
fn every_task_id_named_in_the_tasks_comment_is_on_the_list() {
    let text = schema("tasks.capnp");
    let line = text
        .lines()
        .find(|l| l.contains("# root capability (capability ids:"))
        .unwrap();
    let tail: String = text
        .split(line)
        .nth(1)
        .unwrap()
        .lines()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ");
    let all = format!("{line} {tail}");
    for id in [
        "task.create",
        "task.edit",
        "task.delete",
        "task.claim",
        "relation.create",
    ] {
        assert!(all.contains(id), "{id} in the comment");
        assert!(capabilities::is_known(id), "{id}");
    }
}

// ---- tasks.capnp: claim / release ----

#[test]
fn claim_and_release_are_slots_17_and_18_without_a_by() {
    let text = schema("tasks.capnp");
    let claim = text
        .lines()
        .find(|l| l.trim_start().starts_with("claim "))
        .unwrap();
    let release = text
        .lines()
        .find(|l| l.trim_start().starts_with("release "))
        .unwrap();
    assert!(
        claim.contains("@17 (id :Uuid, revision :UInt64, until :Timestamp) -> (task :Task)"),
        "{claim}"
    );
    assert!(
        release.contains("@18 (id :Uuid, revision :UInt64) -> (task :Task)"),
        "{release}"
    );
    for l in [claim, release] {
        let code = l.split('#').next().unwrap();
        assert!(
            !code.contains("by"),
            "no `by`: the caller is the connection"
        );
    }
    assert!(!text.contains("addNote"), "no addNote");
    // the earlier slots keep their place
    assert!(text.contains("createRelationWithGap @16 ("));
    assert!(text.contains("dependents     @15 ("));
}
