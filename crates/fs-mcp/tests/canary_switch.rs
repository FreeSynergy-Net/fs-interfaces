//! The switch of the server and the vault: off offers nothing and the server does not start; the vault is never on in production.
//! Pattern: none (tests).
#![cfg(feature = "conformance")]

mod support;

use std::rc::Rc;

use capnp_compiler::mcpgen::rt::{Never, Off, Switch};
use fs_mcp::serve::{StartError, Started};
use support::canary::calls::StoreCalls;
use support::canary::{programs, vault};

#[test]
fn canary_switch_off_offers_nothing_and_the_server_does_not_start() {
    for p in programs() {
        for sw in [&Off as &dyn Switch, &Never] {
            let r = fs_mcp::serve::Server::new(
                StoreCalls::new(Rc::default(), vec![]),
                p.table,
                &p.catalog,
                sw,
                vault::info(),
            );
            assert!(matches!(r, Err(StartError::SwitchOff)), "{}", p.name);
        }
        assert!(
            fs_mcp::serve::Server::new(
                StoreCalls::new(Rc::default(), vec![]),
                p.table,
                &p.catalog,
                &Started,
                vault::info()
            )
            .is_ok()
        );
    }
}

#[test]
fn canary_the_vault_is_never_on_in_production() {
    for release_all in [false, true] {
        let r = vault::production_server(
            StoreCalls::new(Rc::default(), vec![]),
            &vault::catalog(release_all),
        );
        assert!(
            matches!(r, Err(StartError::SwitchOff)),
            "release_all {release_all}"
        );
    }
    assert!(!Never.is_on());
    assert_eq!(
        std::mem::size_of::<Never>(),
        0,
        "a type without state: nothing to set"
    );
    // the only switch of this crate that is on is `Started`; a second one must be seen by a human
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut ons = Vec::new();
    let mut stack = vec![dir];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(d).unwrap().map(Result::unwrap) {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                let text = std::fs::read_to_string(&p).unwrap();
                let n = text.matches("fn is_on").count();
                if n > 0 {
                    ons.push((p.file_name().unwrap().to_string_lossy().into_owned(), n));
                }
            }
        }
    }
    assert_eq!(
        ons,
        [("switch.rs".to_owned(), 1)],
        "the switches of this crate: only Started"
    );
    // a vault in the test is on, and only through the test path
    assert!(
        vault::test_server(
            StoreCalls::new(Rc::default(), vec![]),
            &vault::catalog(false)
        )
        .is_ok()
    );
}
