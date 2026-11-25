//! S4-B1: `ChangesQuery.scope`/`form` and `ModuleCore::follows` (appended; the default keeps the old behaviour). Needs feature `fake`.
#![cfg(feature = "fake")]

use fs_interfaces::module::fake::OwnerOnlyModule;
use fs_interfaces::module::{
    Actor, Applied, Blob, Caller, ChangeFeed, ChangeKind, ChangePage, ChangeRef, ChangesQuery,
    Context, Follow, Invocation, ModuleCore, Origin, Page, PageQuery, Refusal, RightsRules,
    StepRef,
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
    assert_eq!((q.scope.as_str(), q.form.as_str()), ("b1", "cards-in-columns@1.4.0"));
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
    assert!(m.follows(&cx(), "", "").expect("all").admits(&change("b2/c1")));
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
    let s = Scoped(OwnerOnlyModule::new()).follows(&cx(), "", "b1").expect("s");
    assert!(d.admits(&change("b2/c1")) && !s.admits(&change("b2/c1")));
}

#[test]
fn follow_debug_names_the_kind_not_the_closure() {
    assert_eq!(format!("{:?}", Follow::all()), "Follow(\"all\")");
    assert_eq!(format!("{:?}", Follow::by(|_| true)), "Follow(\"by\")");
}
