//! SW-1 (Schreibweg, ENTSCHEID-schnitt section 3), part 2 (part 1: `write_forms.rs`): `RecordPage.may @5`, `FormSource.invoke` with `result`, the capabilities
//! `task.*` and `Items.claim @17` / `release @18` (no `by`). Everything is appended: the old slots stay as they were.

use std::path::{Path, PathBuf};

use fs_interfaces::capabilities::{self, name};
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
    // WriteResult itself
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

fn flat(msg: &capnp::message::Builder<capnp::message::HeapAllocator>) -> Vec<u8> {
    capnp::serialize::write_message_to_words(msg)
}

#[test]
fn invoke_result_old_service_to_new_reader_and_new_service_to_old_reader() {
    // an old service answers without `result`: the new reader sees `failed` and `problem` and an empty `result`
    let mut old_msg = capnp::message::Builder::new_default();
    {
        let mut r =
            old_msg.init_root::<formsource_capnp::form_source::invoke_results::Builder<'_>>();
        r.set_failed(true);
        r.reborrow().init_problem().set_title("nope");
    }
    let bytes = flat(&old_msg);
    let reader = capnp::serialize::read_message_from_flat_slice(
        &mut &bytes[..],
        capnp::message::ReaderOptions::new(),
    )
    .unwrap();
    let r = reader
        .get_root::<new::form_source::invoke_results::Reader<'_>>()
        .unwrap();
    assert!(r.get_failed());
    assert_eq!(
        r.get_problem()
            .unwrap()
            .get_title()
            .unwrap()
            .to_str()
            .unwrap(),
        "nope"
    );
    assert!(!r.has_result());
    assert_eq!(r.get_result().unwrap().get_record().unwrap().len(), 0);

    // a new service sets `result`: the old reader reads `failed` and `problem` without error and ignores `result`
    let mut new_msg = capnp::message::Builder::new_default();
    {
        let mut r = new_msg.init_root::<new::form_source::invoke_results::Builder<'_>>();
        r.set_failed(false);
        r.reborrow().init_problem().set_title("fine");
        let mut res = r.init_result();
        res.set_record("task-1");
        res.set_text("created");
    }
    let bytes = flat(&new_msg);
    let reader = capnp::serialize::read_message_from_flat_slice(
        &mut &bytes[..],
        capnp::message::ReaderOptions::new(),
    )
    .unwrap();
    let r = reader
        .get_root::<formsource_capnp::form_source::invoke_results::Reader<'_>>()
        .unwrap();
    assert!(!r.get_failed());
    assert_eq!(
        r.get_problem()
            .unwrap()
            .get_title()
            .unwrap()
            .to_str()
            .unwrap(),
        "fine"
    );
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

#[test]
fn the_value_commands_are_on_the_list_for_any_module() {
    for n in [
        name::VALUE_GET,
        name::VALUE_SET,
        name::VALUE_RESET,
        name::VALUE_LOCK,
        name::VALUE_UNLOCK,
        name::VALUE_LOG,
    ] {
        assert!(capabilities::is_known(n), "{n}");
        assert!(capabilities::is_planned(n), "{n} has no interface yet");
        assert!(n.starts_with("value."));
    }
    // The host prefixes the module name; the list holds the bare key only.
    assert!(!capabilities::is_known("settings.value.set"));
}
