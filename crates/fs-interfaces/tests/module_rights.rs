//! Stufe 4, part 1: the fake module "only the owner may change" under the rules of the contract (needs feature `fake`).
#![cfg(feature = "fake")]

use fs_interfaces::module::fake::{CREATE, OwnerOnlyModule, SET};
use fs_interfaces::module::{Actor, Ask, Caller, Claimed, Context, ModuleCore, Verdict};
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
fn only_the_owner_may_change() {
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

#[test]
fn introduce_changes_only_claimed() {
    let a = cx("urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e01");
    let b = a.clone().with_claimed(Claimed::new("settings", "w1", ""));
    assert_eq!(a.person, b.person);
    assert_eq!(b.claimed.program, "settings");
    assert_eq!(a.claimed, Claimed::default());
}
