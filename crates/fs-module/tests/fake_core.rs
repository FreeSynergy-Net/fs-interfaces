//! `FakeCore` and `FakeRules` (Stufe 4, A1): the fake as a `ModuleCore`, its rights apart from its effect, and the one body shared with `FakeSource`.
//! One test per case of the decision S4-3, package A1.

mod common;

use common::{FORM, call, cx, page_query};
use fs_interfaces::module::{Ask, HiddenReason, ModuleCore, Refusal, RightsRules, Verdict};
use fs_module::{FakeCore, FakeRules, FakeSource, Mode};
use fs_types::Utc;

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
