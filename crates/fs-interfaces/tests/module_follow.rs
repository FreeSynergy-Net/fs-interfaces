//! S4-B1: `ChangesQuery.scope`/`form` and `ModuleCore::follows` (appended; the default keeps the old behaviour). Needs feature `fake`.
//! Stufe 4 V1: `resolve_scope`, `ChangeRef.scopes`, `Follow::in_scope`, `Refusal::unknown_scope`, `FeedLoss`/`on_feed_break`, `FieldClash.record`.
#![cfg(feature = "fake")]

use std::cell::Cell;
use std::future::Future;
use std::pin::pin;
use std::rc::Rc;
use std::task::{Context as TaskCx, Poll, Waker};

use fs_interfaces::module::fake::OwnerOnlyModule;
use fs_interfaces::module::{
    Actor, Applied, Blob, Caller, ChangeFeed, ChangeKind, ChangePage, ChangeRef, ChangesQuery,
    Context, FeedBreak, FeedLoss, FieldClash, Follow, Invocation, ModuleCore, Origin, Page,
    PageQuery, Refusal, RightsRules, StepRef,
};
use fs_interfaces::owner::fake::person;
use fs_types::Utc;

fn cx() -> Context {
    let caller = Caller::Connection {
        system_id: "1000".into(),
        system_name: None,
    };
    Context::from_connection(
        caller,
        Actor::Person,
        person("urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e01"),
    )
}

fn change(object: &str) -> ChangeRef {
    ChangeRef::new(
        "a1",
        object,
        "2",
        "1",
        Origin::default(),
        ChangeKind::Update,
        Utc::from_millis(0),
    )
}

/// A module with scopes: delegates everything to the fake, overrides only `follows` (scope `b1` = objects `b1/...`).
struct Scoped(OwnerOnlyModule);

impl ModuleCore for Scoped {
    fn level(&self) -> u32 {
        self.0.level()
    }
    fn provider(&self) -> &str {
        self.0.provider()
    }
    fn rules(&self) -> &dyn RightsRules {
        self.0.rules()
    }
    async fn page(&self, cx: &Context, query: PageQuery) -> Result<Page, Refusal> {
        self.0.page(cx, query).await
    }
    async fn blob(&self, cx: &Context, record: &str, field: &str) -> Result<Blob, Refusal> {
        self.0.blob(cx, record, field).await
    }
    async fn apply(&self, cx: &Context, command: Invocation) -> Result<Applied, Refusal> {
        self.0.apply(cx, command).await
    }
    async fn revert(&self, cx: &Context, change: &str, step: StepRef) -> Result<Applied, Refusal> {
        self.0.revert(cx, change, step).await
    }
    async fn changes(&self, cx: &Context, query: ChangesQuery) -> Result<ChangePage, Refusal> {
        self.0.changes(cx, query).await
    }
    fn feed(&self, sink: ChangeFeed) {
        self.0.feed(sink);
    }
    fn follows(&self, _cx: &Context, form: &str, scope: &str) -> Result<Follow, Refusal> {
        if form == "rows@1.0.0" {
            let mut r = Refusal::forbidden(form);
            r.kind = "unknown-form".into();
            r.status = 404;
            return Err(r);
        }
        if scope.is_empty() {
            return Ok(Follow::all());
        }
        let prefix = format!("{scope}/");
        Ok(Follow::by(move |c| c.object.starts_with(&prefix)))
    }
}

#[test]
fn query_default_means_all_and_old_fields_stay() {
    let q = ChangesQuery::default();
    assert!(q.scope.is_empty() && q.form.is_empty());
    assert!(q.run.is_empty() && q.cursor.is_empty() && q.limit == 0);
    let mut q = ChangesQuery::default();
    q.scope = "b1".into();
    q.form = "cards-in-columns@1.4.0".into();
    assert_eq!(
        (q.scope.as_str(), q.form.as_str()),
        ("b1", "cards-in-columns@1.4.0")
    );
}

#[test]
fn follow_all_admits_everything() {
    let f = Follow::all();
    assert!(f.admits(&change("x")) && f.admits(&change("")));
    assert!(Follow::default().admits(&change("y")));
}

#[test]
fn follow_by_admits_only_what_the_test_says() {
    let f = Follow::by(|c| c.object == "keep");
    assert!(f.admits(&change("keep")));
    assert!(!f.admits(&change("drop")));
    assert!(f.clone().admits(&change("keep")) && !f.clone().admits(&change("drop")));
}

#[test]
fn default_follows_keeps_the_old_behaviour() {
    let module = OwnerOnlyModule::new();
    for (form, scope) in [("", ""), ("rows@1.0.0", ""), ("any", "any-scope")] {
        let f = module.follows(&cx(), form, scope).expect("default follows");
        assert!(f.admits(&change("whatever")), "{form}/{scope}");
    }
}

#[test]
fn override_narrows_by_scope_and_other_scopes_hear_nothing() {
    let m = Scoped(OwnerOnlyModule::new());
    let f = m.follows(&cx(), "", "b1").expect("scoped");
    assert!(f.admits(&change("b1/c1")));
    assert!(!f.admits(&change("b2/c1")));
    assert!(
        m.follows(&cx(), "", "")
            .expect("all")
            .admits(&change("b2/c1"))
    );
}

#[test]
fn override_refuses_an_unknown_form() {
    let m = Scoped(OwnerOnlyModule::new());
    let r = m.follows(&cx(), "rows@1.0.0", "").expect_err("refused");
    assert_eq!((r.kind.as_str(), r.status), ("unknown-form", 404));
}

#[test]
fn counter_proof_default_is_not_the_narrow_one() {
    // The default would hide nothing; the override does: the same call differs, so the tests above do not pass by accident.
    let d = OwnerOnlyModule::new().follows(&cx(), "", "b1").expect("d");
    let s = Scoped(OwnerOnlyModule::new())
        .follows(&cx(), "", "b1")
        .expect("s");
    assert!(d.admits(&change("b2/c1")) && !s.admits(&change("b2/c1")));
}

#[test]
fn follow_debug_names_the_kind_not_the_closure() {
    assert_eq!(format!("{:?}", Follow::all()), "Follow(\"all\")");
    assert_eq!(format!("{:?}", Follow::by(|_| true)), "Follow(\"by\")");
}

/// Polls a future that never waits (the defaults have nothing to await).
fn run<F: Future>(f: F) -> F::Output {
    let mut f = pin!(f);
    let mut task = TaskCx::from_waker(Waker::noop());
    match f.as_mut().poll(&mut task) {
        Poll::Ready(out) => out,
        Poll::Pending => panic!("the future must be ready at once"),
    }
}

fn in_scopes(object: &str, scopes: &[&str]) -> ChangeRef {
    change(object).in_scopes(scopes.iter().copied())
}

#[test]
fn in_scope_admits_only_changes_in_that_scope() {
    let f = Follow::in_scope("board-1");
    assert!(f.admits(&in_scopes("c1", &["board-1"])));
    assert!(
        f.admits(&in_scopes("c1", &["board-0", "board-1"])),
        "one of several"
    );
    assert!(!f.admits(&in_scopes("c1", &["board-2"])));
    assert!(
        !f.admits(&change("c1")),
        "a change in no scope is not in board-1"
    );
    assert!(
        !f.admits(&in_scopes("c1", &["board-10"])),
        "no prefix match"
    );
}

#[test]
fn in_scope_empty_id_means_all() {
    let f = Follow::in_scope("");
    assert!(f.admits(&change("x")) && f.admits(&in_scopes("x", &["b"])));
}

#[test]
fn change_ref_scopes_builder_and_equality() {
    assert!(change("x").scopes.is_empty(), "new() starts in no scope");
    let c = in_scopes("x", &["b1", "b2"]);
    assert_eq!(c.scopes, ["b1", "b2"]);
    assert_eq!(
        c.clone().in_scopes(["b3"]).scopes,
        ["b3"],
        "replaces, not appends"
    );
    assert_ne!(
        c,
        change("x"),
        "PartialEq compares scopes (Risiko 1 of the decision)"
    );
    assert_eq!(c, c.clone());
}

#[test]
fn default_resolve_scope_passes_through() {
    let module = OwnerOnlyModule::new();
    for scope in ["", "b1", "a name"] {
        assert_eq!(
            run(module.resolve_scope(&cx(), "f@1.0.0", scope)).as_deref(),
            Ok(scope)
        );
    }
}

/// A module with scopes: resolves a name to the fixed id; any other scope is unknown.
struct Boards(OwnerOnlyModule);

impl ModuleCore for Boards {
    fn level(&self) -> u32 {
        self.0.level()
    }
    fn provider(&self) -> &str {
        self.0.provider()
    }
    fn rules(&self) -> &dyn RightsRules {
        self.0.rules()
    }
    async fn page(&self, cx: &Context, query: PageQuery) -> Result<Page, Refusal> {
        self.0.page(cx, query).await
    }
    async fn blob(&self, cx: &Context, record: &str, field: &str) -> Result<Blob, Refusal> {
        self.0.blob(cx, record, field).await
    }
    async fn apply(&self, cx: &Context, command: Invocation) -> Result<Applied, Refusal> {
        self.0.apply(cx, command).await
    }
    async fn revert(&self, cx: &Context, change: &str, step: StepRef) -> Result<Applied, Refusal> {
        self.0.revert(cx, change, step).await
    }
    async fn changes(&self, cx: &Context, query: ChangesQuery) -> Result<ChangePage, Refusal> {
        self.0.changes(cx, query).await
    }
    fn feed(&self, sink: ChangeFeed) {
        self.0.feed(sink);
    }
    async fn resolve_scope(
        &self,
        _cx: &Context,
        _form: &str,
        scope: &str,
    ) -> Result<String, Refusal> {
        match scope {
            "" => Ok(String::new()),
            "Todo" | "id-todo" => Ok("id-todo".into()),
            other => Err(Refusal::unknown_scope(other)),
        }
    }
    fn follows(&self, _cx: &Context, _form: &str, scope: &str) -> Result<Follow, Refusal> {
        Ok(Follow::in_scope(scope))
    }
}

#[test]
fn override_resolves_a_name_to_the_id_and_refuses_the_unknown() {
    let m = Boards(OwnerOnlyModule::new());
    assert_eq!(
        run(m.resolve_scope(&cx(), "", "Todo")).as_deref(),
        Ok("id-todo")
    );
    assert_eq!(run(m.resolve_scope(&cx(), "", "")).as_deref(), Ok(""));
    let r = run(m.resolve_scope(&cx(), "", "nowhere")).expect_err("unknown");
    assert_eq!(
        (r.kind.as_str(), r.status, r.detail.as_str()),
        ("unknown-scope", 404, "nowhere")
    );
    let f = m.follows(&cx(), "", "id-todo").expect("follows");
    assert!(f.admits(&in_scopes("c1", &["id-todo"])) && !f.admits(&in_scopes("c1", &["id-done"])));
}

#[test]
fn unknown_scope_refusal_has_no_conflict() {
    let r = Refusal::unknown_scope("x");
    assert_eq!((r.kind.as_str(), r.status), ("unknown-scope", 404));
    assert!(r.conflict.is_none());
}

#[test]
fn default_on_feed_break_does_nothing() {
    let hits = Rc::new(Cell::new(0_u32));
    let h = Rc::clone(&hits);
    let sink: FeedBreak = Box::new(move |_| h.set(h.get() + 1));
    OwnerOnlyModule::new().on_feed_break(sink);
    assert_eq!(
        hits.get(),
        0,
        "the default registers nothing and calls nothing"
    );
}

#[test]
fn feed_loss_values() {
    assert_eq!(
        FeedLoss::Lagged { missed: 3 },
        FeedLoss::Lagged { missed: 3 }
    );
    assert_ne!(FeedLoss::Lagged { missed: 3 }, FeedLoss::Closed);
    let sink: FeedBreak = Box::new(|loss| assert_eq!(loss, FeedLoss::Closed));
    sink(FeedLoss::Closed);
}

#[test]
fn field_clash_record_builder() {
    let c = FieldClash::new("text", "now", "mine");
    assert!(c.record.is_empty(), "new() leaves the record unknown");
    let c = c.on("note-1");
    assert_eq!((c.record.as_str(), c.path.as_str()), ("note-1", "text"));
    assert_ne!(c, FieldClash::new("text", "now", "mine"));
}
