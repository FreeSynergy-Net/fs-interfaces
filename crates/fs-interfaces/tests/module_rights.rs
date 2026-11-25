//! Stufe 4, part 1: the fake module "only the owner may change" under the rules of the contract (needs feature `fake`).
#![cfg(feature = "fake")]

use fs_interfaces::module::fake::{CREATE, OwnerOnlyModule, SET};
use std::future::Future;
use std::pin::pin;
use std::task::{Context as TaskCx, Poll, Waker};

use fs_interfaces::module::{
    Actor, Ask, Caller, ChangeKind, ChangeRef, Claimed, Conflict, Context, FieldClash, HiddenField,
    HiddenReason, Invocation, ModuleCore, Origin, Refusal, StepRef, Value, Verdict,
};
use fs_interfaces::owner::fake::person;
use fs_types::Utc;

fn cx(who: &str) -> Context {
    let caller = Caller::Connection {
        system_id: "1000".into(),
        system_name: None,
    };
    Context::from_connection(caller, Actor::Person, person(who))
}

#[test]
fn create_is_free_unknown_record_denied() {
    let module = OwnerOnlyModule::new();
    let alice = cx("urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e01");
    let bob = cx("urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e02");
    let at = Utc::from_millis(0);
    assert!(
        module
            .rules()
            .may(&Ask::new(&bob, CREATE, None, at))
            .is_allowed()
    );
    // Unknown record: denied for everybody.
    assert_eq!(
        module
            .rules()
            .may(&Ask::new(&alice, SET, Some("note-1"), at)),
        Verdict::deny("unknown-record")
    );
}

/// Polls a future that never waits (the fake has nothing to await).
fn run<F: Future>(f: F) -> F::Output {
    let mut f = pin!(f);
    let mut task = TaskCx::from_waker(Waker::noop());
    match f.as_mut().poll(&mut task) {
        Poll::Ready(out) => out,
        Poll::Pending => panic!("the fake future must be ready at once"),
    }
}

#[test]
fn non_owner_is_denied_owner_is_allowed() {
    let module = OwnerOnlyModule::new();
    let alice = cx("urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e01");
    let bob = cx("urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e02");
    let at = Utc::from_millis(0);
    let mut create = Invocation::default();
    create.command = CREATE.into();
    create.args = vec![Value::new("text", "hello")];
    let applied = run(module.apply(&alice, create)).expect("create");
    assert_eq!(applied.record, "note-1");
    assert_eq!(
        module.rules().may(&Ask::new(&bob, SET, Some("note-1"), at)),
        Verdict::deny("not-owner")
    );
    assert_eq!(
        module
            .rules()
            .may(&Ask::new(&alice, SET, Some("note-1"), at)),
        Verdict::Allow
    );
    assert_eq!(module.rules().hidden(&bob, "note-1", "text"), None);
}

#[test]
fn constructors_build_the_values_from_outside() {
    let origin = Origin::new("urn:uuid:1", "settings", "w1", "");
    let at = Utc::from_millis(5);
    let c = ChangeRef::new(
        "a-1",
        "note-1",
        "2",
        "1",
        origin.clone(),
        ChangeKind::Update,
        at,
    );
    assert_eq!((c.undoes.as_deref(), c.kind), (None, ChangeKind::Update));
    let u = ChangeRef::new("a-2", "note-1", "3", "2", origin, ChangeKind::Undo, at).undoing("a-1");
    assert_eq!(u.undoes.as_deref(), Some("a-1"));
    assert_eq!(ChangeKind::Undo.as_str(), "Undo");
    let clash = FieldClash::new("text", "now", "mine");
    let r = Refusal::conflict("note-1", Conflict::new(c, vec![clash]));
    assert_eq!((r.kind.as_str(), r.status), ("conflict", 409));
    assert_eq!(r.conflict.expect("typed").fields[0].mine, "mine");
    let h = HiddenField::new("secret", HiddenReason::NoRight);
    assert_eq!(h.path, "secret");
    assert_eq!(StepRef::new("run", "s1").step, "s1");
}

#[test]
fn introduce_changes_only_claimed() {
    let a = cx("urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e01");
    let b = a.clone().with_claimed(Claimed::new("settings", "w1", ""));
    assert_eq!(a.person, b.person);
    assert_eq!(b.claimed.program, "settings");
    assert_eq!(a.claimed, Claimed::default());
}
