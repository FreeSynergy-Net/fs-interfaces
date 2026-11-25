//! `FakeCore` writing (Stufe 4, A1): the effect without rights, steps, conflict, undo, changes and the feed.

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use common::{FORM, call, cx, page_query, with_step};
use fs_interfaces::module::{ChangeKind, ChangesQuery, ModuleCore, StepRef};
use fs_module::{FakeCore, Mode};

// 10-14: writing, steps, conflict, undo.

#[tokio::test(flavor = "current_thread")]
async fn apply_records_one_change_with_origin_from_the_context() {
    let (core, c) = (FakeCore::new(Mode::Strict), cx("net.example.prog"));
    let done = core
        .apply(
            &c,
            call(
                "card.create",
                "",
                &[("title", "T"), ("column", "todo"), ("lane", "main")],
            ),
        )
        .await
        .unwrap();
    let ch = done.change.expect("a change is recorded");
    assert_eq!(
        (ch.kind, ch.object.as_str(), ch.before.as_str()),
        (ChangeKind::Create, done.record.as_str(), "")
    );
    assert_eq!(
        (ch.origin.program.as_str(), ch.origin.instance.as_str()),
        ("net.example.prog", "i1")
    );
    assert_eq!(ch.activity.len(), 36, "a UUID as text");
}

#[tokio::test(flavor = "current_thread")]
async fn same_step_acts_once_and_ignore_step_acts_twice() {
    let c = cx("p");
    let mv = || with_step(call("card.move", "c1", &[("column", "done")]), "r", "s1");
    let strict = FakeCore::new(Mode::Strict);
    let (a, b) = (
        strict.apply(&c, mv()).await.unwrap(),
        strict.apply(&c, mv()).await.unwrap(),
    );
    assert_eq!(a.change.unwrap().activity, b.change.unwrap().activity);
    let lax = FakeCore::new(Mode::IgnoreStep);
    let (a, b) = (
        lax.apply(&c, mv()).await.unwrap(),
        lax.apply(&c, mv()).await.unwrap(),
    );
    assert_ne!(a.change.unwrap().activity, b.change.unwrap().activity);
}

#[tokio::test(flavor = "current_thread")]
async fn stale_expect_is_a_conflict_with_the_change_in_between() {
    let (core, c) = (FakeCore::new(Mode::Strict), cx("p"));
    core.apply(&c, call("card.move", "c1", &[("column", "done")]))
        .await
        .unwrap();
    let mut stale = call("card.move", "c1", &[("column", "todo")]);
    stale.expect = Some("1".into());
    let e = core.apply(&c, stale).await.unwrap_err();
    assert_eq!((e.kind.as_str(), e.status), ("conflict", 409));
    let conflict = e.conflict.expect("typed conflict");
    assert_eq!(conflict.fields[0].path, "column");
    assert_eq!(conflict.by.object, "c1");
    assert_eq!(
        conflict.fields[0].record, "c1",
        "the clash names its record (V1: the wrapper's scrub needs it)"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn revert_makes_a_new_undo_change_and_a_second_revert_is_refused() {
    let (core, c) = (FakeCore::new(Mode::Strict), cx("p"));
    let done = core
        .apply(&c, call("card.move", "c1", &[("column", "done")]))
        .await
        .unwrap();
    let id = done.change.unwrap().activity;
    let undo = core.revert(&c, &id, StepRef::new("u", "1")).await.unwrap();
    let ch = undo.change.unwrap();
    assert_eq!(
        (ch.kind, ch.undoes.as_deref()),
        (ChangeKind::Undo, Some(id.as_str()))
    );
    let page = core.page(&c, page_query(FORM, "", "", 0)).await.unwrap();
    let col = |r: &fs_interfaces::module::RecordView| {
        r.values
            .iter()
            .find(|v| v.path == "column")
            .unwrap()
            .text
            .clone()
    };
    assert_eq!(col(&page.records[0]), "todo");
    let again = core
        .revert(&c, &id, StepRef::new("u", "2"))
        .await
        .unwrap_err();
    assert_eq!(
        again.status, 409,
        "the record changed after the change: conflict"
    );
    let unknown = core
        .revert(&c, "not-a-uuid", StepRef::default())
        .await
        .unwrap_err();
    assert_eq!(
        (unknown.kind.as_str(), unknown.status),
        ("unknown-change", 404)
    );
}

// 15-17: changes and feed.

#[tokio::test(flavor = "current_thread")]
async fn changes_list_by_program_or_run_newest_first_with_a_cursor() {
    let core = FakeCore::new(Mode::Strict);
    let (a, b) = (cx("prog-a"), cx("prog-b"));
    core.apply(
        &a,
        with_step(call("card.move", "c1", &[("column", "done")]), "run1", "s"),
    )
    .await
    .unwrap();
    core.apply(&a, call("card.move", "c2", &[("column", "done")]))
        .await
        .unwrap();
    core.apply(&b, call("card.delete", "c2", &[]))
        .await
        .unwrap();
    let mine = core.changes(&a, ChangesQuery::default()).await.unwrap();
    assert_eq!(
        mine.changes
            .iter()
            .map(|c| c.object.as_str())
            .collect::<Vec<_>>(),
        ["c2", "c1"]
    );
    let mut q = ChangesQuery::default();
    q.run = "run1".into();
    assert_eq!(core.changes(&b, q).await.unwrap().changes.len(), 1);
    let mut q = ChangesQuery::default();
    q.limit = 1;
    let first = core.changes(&a, q).await.unwrap();
    assert_eq!((first.changes.len(), first.next.as_str()), (1, "1"));
}

#[tokio::test(flavor = "current_thread")]
async fn feed_hears_every_new_change_once_a_replay_is_silent() {
    let (core, c) = (FakeCore::new(Mode::Strict), cx("p"));
    let heard = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&heard);
    core.feed(Box::new(move |ch| {
        sink.borrow_mut().push(ch.activity.clone());
    }));
    let mv = || with_step(call("card.move", "c1", &[("column", "done")]), "r", "s1");
    let first = core.apply(&c, mv()).await.unwrap();
    core.apply(&c, mv()).await.unwrap();
    let undo = core
        .revert(
            &c,
            &first.change.as_ref().unwrap().activity,
            StepRef::new("u", "1"),
        )
        .await
        .unwrap();
    let heard = heard.borrow();
    assert_eq!(heard.len(), 2, "the replay is not a new change");
    assert_eq!(heard[1], undo.change.unwrap().activity);
}

#[tokio::test(flavor = "current_thread")]
async fn a_refused_call_is_not_a_change() {
    let (core, c) = (FakeCore::new(Mode::Strict), cx("p"));
    let heard = Rc::new(RefCell::new(0));
    let n = Rc::clone(&heard);
    core.feed(Box::new(move |_| *n.borrow_mut() += 1));
    core.apply(&c, call("card.move", "c1", &[]))
        .await
        .unwrap_err();
    assert_eq!(*heard.borrow(), 0);
    let none = core.changes(&c, ChangesQuery::default()).await.unwrap();
    assert_eq!(none.changes.len(), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn resolve_scope_is_strict_and_lax_scope_passes_through() {
    let c = cx("p");
    let strict = FakeCore::new(Mode::Strict);
    assert_eq!(strict.resolve_scope(&c, FORM, "").await.as_deref(), Ok(""));
    let e = strict.resolve_scope(&c, FORM, "x").await.unwrap_err();
    assert_eq!((e.kind.as_str(), e.status), ("unknown-scope", 404));
    let lax = FakeCore::new(Mode::LaxScope);
    assert_eq!(lax.resolve_scope(&c, FORM, "x").await.as_deref(), Ok("x"));
}

#[tokio::test(flavor = "current_thread")]
async fn follows_is_in_scope_and_the_empty_scope_hears_the_feed() {
    let (core, c) = (FakeCore::new(Mode::Strict), cx("p"));
    let heard = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&heard);
    core.feed(Box::new(move |ch| sink.borrow_mut().push(ch.clone())));
    core.apply(&c, call("card.move", "c1", &[("column", "done")]))
        .await
        .unwrap();
    let change = heard.borrow()[0].clone();
    assert!(change.scopes.is_empty(), "the fake has no scopes");
    assert!(core.follows(&c, "", "").unwrap().admits(&change));
    assert!(!core.follows(&c, "", "b1").unwrap().admits(&change));
}
