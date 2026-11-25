//! `FakeCore` and `FakeRules` (Stufe 4, A1): the fake as a `ModuleCore`, its rights apart from its effect, and the one body shared with `FakeSource`.
//! One test per case of the decision S4-3, package A1.

use std::cell::RefCell;
use std::rc::Rc;

use fs_interfaces::module::{
    Actor, Ask, Caller, ChangeKind, ChangesQuery, Context, HiddenReason, Invocation, ModuleCore,
    PageQuery, RightsRules, StepRef, Value, Verdict,
};
use fs_interfaces::module::{Claimed, Refusal};
use fs_interfaces::owner::fake::person;
use fs_module::{FakeCore, FakeRules, FakeSource, Mode};
use fs_types::Utc;

fn cx(program: &str) -> Context {
    let caller = Caller::Connection {
        system_id: "1000".into(),
        system_name: None,
    };
    Context::from_connection(
        caller,
        Actor::Person,
        person("urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e01"),
    )
    .with_claimed(Claimed::new(program, "i1", "d1"))
}

fn call(command: &str, record: &str, args: &[(&str, &str)]) -> Invocation {
    let mut c = Invocation::default();
    c.command = command.into();
    c.record = (!record.is_empty()).then(|| record.to_owned());
    c.args = args.iter().map(|(p, t)| Value::new(p, t)).collect();
    c
}

fn with_step(mut c: Invocation, run: &str, step: &str) -> Invocation {
    c.step = Some(StepRef::new(run, step));
    c
}

fn page_query(form: &str, scope: &str, cursor: &str, limit: u32) -> PageQuery {
    let mut q = PageQuery::default();
    q.form = form.into();
    q.scope = scope.into();
    q.cursor = cursor.into();
    q.limit = limit;
    q
}

const FORM: &str = "cards-in-columns@1.0.0";

// 1-3: the rights are in FakeRules, not in the module.

#[tokio::test(flavor = "current_thread")]
async fn core_acts_without_asking_rights_the_rules_deny_the_same_call() {
    let (core, c) = (FakeCore::new(Mode::Strict), cx("p"));
    let at = Utc::from_millis(0);
    assert_eq!(
        core.rules().may(&Ask::new(&c, "card.move", Some("c3"), at)),
        Verdict::deny("locked")
    );
    // The same call on the core itself runs: a module never checks rights (the wrapper asks the rules first).
    let done = core
        .apply(&c, call("card.move", "c3", &[("column", "todo")]))
        .await
        .expect("the core does not check rights");
    assert_eq!(done.record, "c3");
}

#[test]
fn rules_deny_the_locked_card_and_the_locked_set_command_and_allow_the_rest() {
    let (rules, c) = (FakeRules, cx("p"));
    let at = Utc::from_millis(0);
    let may = |cmd: &str, rec: Option<&str>| rules.may(&Ask::new(&c, cmd, rec, at));
    assert_eq!(may("card.move", Some("c3")), Verdict::deny("locked"));
    assert_eq!(may("card.delete", Some("c3")), Verdict::deny("locked"));
    assert_eq!(may("card.clear", None), Verdict::deny("locked"));
    assert!(may("card.move", Some("c1")).is_allowed());
    assert!(may("card.create", None).is_allowed());
    assert!(may("change.revert", Some("c3")).is_allowed());
    assert!(may("change.watch", None).is_allowed());
    assert!(
        may("no.such.command", Some("c3")).is_allowed(),
        "unknown = allow, the core refuses it"
    );
}

#[test]
fn rules_hide_nothing() {
    let c = cx("p");
    assert_eq!(FakeRules.hidden(&c, "c1", "title"), None::<HiddenReason>);
}

// 4-5: FakeSource keeps its level 1 way: rights, then effect.

#[test]
fn fake_source_run_is_rights_then_effect_as_before() {
    let src = FakeSource::new(Mode::Strict);
    let args = [("column".to_owned(), "done".to_owned())];
    assert_eq!(
        src.try_invoke("card.move", "c3", &args).unwrap_err().status,
        403
    );
    assert_eq!(
        src.try_invoke("card.clear", "", &[]).unwrap_err().status,
        403
    );
    assert_eq!(
        src.try_invoke("card.nope", "c1", &[]).unwrap_err().status,
        404
    );
    assert_eq!(src.try_invoke("card.move", "c1", &args).unwrap().1, "moved");
}

#[tokio::test(flavor = "current_thread")]
async fn core_refuses_unknown_command_and_bad_arguments_as_values() {
    let (core, c) = (FakeCore::new(Mode::Strict), cx("p"));
    let e = core
        .apply(&c, call("card.nope", "c1", &[]))
        .await
        .unwrap_err();
    assert_eq!((e.kind.as_str(), e.status), ("unknown-command", 404));
    let e = core
        .apply(&c, call("card.move", "c1", &[]))
        .await
        .unwrap_err();
    assert_eq!((e.kind.as_str(), e.status), ("bad-arguments", 400));
}

// 6-9: reading.

#[tokio::test(flavor = "current_thread")]
async fn page_gives_rows_total_cursor_and_values_like_the_wire_fake() {
    let (core, c) = (FakeCore::new(Mode::Strict), cx("p"));
    let p = core.page(&c, page_query(FORM, "", "", 2)).await.unwrap();
    assert_eq!(
        (p.records.len(), p.total, p.next.as_str()),
        (2, Some(3), "2")
    );
    assert!(
        p.may.is_empty() && p.records[0].may.is_empty(),
        "the wrapper fills may, not the module"
    );
    let c1 = &p.records[0];
    assert!(
        c1.values
            .iter()
            .any(|v| v.path == "due" && v.text == "2026-11-30T12:00:00+01:00")
    );
    assert_eq!(c1.values.iter().filter(|v| v.path == "assignee").count(), 2);
    let last = core
        .page(&c, page_query("cards-in-columns@1.0", "", "2", 2))
        .await
        .unwrap();
    assert_eq!((last.records.len(), last.next.as_str()), (1, ""));
}

#[tokio::test(flavor = "current_thread")]
async fn page_refusals_are_values_with_the_bare_kind() {
    let (core, c) = (FakeCore::new(Mode::Strict), cx("p"));
    let kind = |r: Result<_, Refusal>| r.map(|_: fs_interfaces::module::Page| ()).unwrap_err();
    assert_eq!(
        kind(core.page(&c, page_query("nope@1.0.0", "", "", 0)).await).kind,
        "unknown-form"
    );
    assert_eq!(
        kind(core.page(&c, page_query(FORM, "x", "", 0)).await).kind,
        "unknown-scope"
    );
    let bad = kind(core.page(&c, page_query(FORM, "", "garbage", 0)).await);
    assert_eq!((bad.kind.as_str(), bad.status), ("bad-arguments", 400));
}

#[tokio::test(flavor = "current_thread")]
async fn blob_is_unsupported_unless_built_with_an_image() {
    let c = cx("p");
    let none = FakeCore::new(Mode::Strict);
    assert_eq!(none.blob(&c, "c1", "image").await.unwrap_err().status, 501);
    let img = FakeCore::new(Mode::Strict).with_blob();
    assert_eq!(
        img.blob(&c, "c1", "image").await.unwrap().bytes,
        fs_module::IMAGE
    );
    assert_eq!(img.blob(&c, "c2", "image").await.unwrap_err().status, 404);
}

#[tokio::test(flavor = "current_thread")]
async fn core_speaks_level_2_as_provider_fake() {
    let core = FakeCore::new(Mode::Strict);
    assert_eq!((core.level(), core.provider()), (2, "fake"));
}

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
        sink.borrow_mut().push(ch.activity.clone())
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
    assert!(
        core.changes(&c, ChangesQuery::default())
            .await
            .unwrap()
            .changes
            .is_empty()
    );
}
