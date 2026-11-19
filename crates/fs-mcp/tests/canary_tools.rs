//! The canary test against the tools of this repository: markers in every place where a secret would sit, every tool called
//! the way a client can call it, every answer, error text and log searched for them. Pattern: none (tests).
//! It does not stop at the first leak: it lists ALL of them (`FSCANARY_FEHLERLISTE` names the file) and then fails.
#![cfg(feature = "conformance")]

mod support;

use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

use capnp_compiler::mcpgen::rt::{Never, Off, Switch};
use fs_mcp::canary::{Canary, CanaryTable, Class};
use fs_mcp::serve::{StartError, Started};
use serde_json::{Value, json};
use support::canary::calls::StoreCalls;
use support::canary::{Findings, Prog, branch, drive, setup, tasks, vault, vault_prog};

fn programs() -> Vec<Prog> {
    vec![tasks(), vault_prog(false)]
}

fn table_json(t: &CanaryTable) -> String {
    json!(
        t.all()
            .iter()
            .map(
                |c| json!({"path": c.path, "marker": c.marker, "secret": c.class == Class::Secret})
            )
            .collect::<Vec<_>>()
    )
    .to_string()
}

fn table_from_json(text: &str) -> CanaryTable {
    let v: Vec<Value> = serde_json::from_str(text).expect("the table");
    CanaryTable::from_canaries(
        v.iter()
            .map(|e| Canary {
                path: e["path"].as_str().unwrap().to_owned(),
                marker: e["marker"].as_str().unwrap().to_owned(),
                class: if e["secret"] == true {
                    Class::Secret
                } else {
                    Class::Sensitive
                },
            })
            .collect(),
    )
}

/// The child process: runs the whole driver with the markers of its parent and writes nothing itself. Whatever it prints
/// (stdout, stderr: logs, panic messages) is searched by the parent.
#[test]
#[ignore = "run by canary_everything in a child process"]
fn child_logs_run() {
    let Ok(text) = std::env::var("FSCANARY_TABLE") else {
        return;
    };
    let table = Rc::new(RefCell::new(table_from_json(&text)));
    let mut f = Findings::default();
    for p in &programs() {
        drive(&mut f, p, &table);
    }
}

fn logs_of_a_child(table: &CanaryTable, f: &mut Findings) {
    let out = Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--exact",
            "child_logs_run",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("FSCANARY_TABLE", table_json(table))
        .output()
        .expect("the child runs");
    f.calls += 1;
    let (stdout, stderr) = (
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(
        stdout.contains("child_logs_run") && out.status.success(),
        "the child ran: {stdout}{stderr}"
    );
    f.check("child process stdout", &stdout, table);
    f.check("child process stderr (logs, panic hook)", &stderr, table);
    f.note(format!(
        "logs: stdout {} bytes and stderr {} bytes of a child process that ran every call",
        stdout.len(),
        stderr.len()
    ));
}

#[test]
fn canary_everything() {
    let progs = programs();
    let table = setup(&progs);
    let mut f = Findings::default();
    for p in &progs {
        drive(&mut f, p, &table);
    }
    let canaries = CanaryTable::from_canaries(table.borrow().all().to_vec());
    logs_of_a_child(&canaries, &mut f);
    // not empty-handed: the stand-in vault has places for four secrets, and the tasks tools have none that is not released
    for place in [
        "vault.get.value",
        "vault.export.blob",
        "vault.rotate.key",
        "vault.token.token",
        "store.EncryptionKey.key",
    ] {
        assert!(canaries.marker(place).is_some(), "a canary in {place}");
    }
    f.note(format!(
        "planted: {} canaries; released places (not planted): {:?}",
        canaries.all().len(),
        progs[0].released
    ));
    // every tool with a secret in an argument or a result is no tool: the table of the generator has no such tool either
    for c in fs_mcp::tasks_cli::COMMANDS {
        for p in c.params {
            if p.secret {
                assert!(
                    fs_mcp::tasks_mcp::TOOLS.iter().all(|t| t.key != c.key),
                    "{} takes a secret ({}) and is a tool",
                    c.key,
                    p.path
                );
            }
        }
    }
    let planted = canaries.all().to_vec();
    let path = f.write(&branch(), &planted);
    assert!(
        f.list.is_empty(),
        "{} leaks, the whole list is in {}:\n{}",
        f.list.len(),
        path.display(),
        f.report(&branch(), &planted)
    );
}

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

#[test]
fn canary_the_driver_finds_a_leak_that_is_there() {
    // a human wrongly released every place of the vault: the screen lets the secrets through, the driver must see them
    let progs = vec![vault_prog(true)];
    let table = setup(&progs);
    let mut f = Findings::default();
    drive(&mut f, &progs[0], &table);
    assert_ne!(progs[0].catalog.tools.len(), 0);
    let paths: std::collections::BTreeSet<&str> = f.list.iter().map(|x| x.path.as_str()).collect();
    for place in [
        "vault.get.value",
        "vault.export.blob",
        "vault.rotate.key",
        "vault.token.token",
    ] {
        assert!(
            paths.contains(place),
            "the leak in {place} is found; found: {paths:?}"
        );
    }
    // in the right form: text as it is, base64 for the data
    let forms: std::collections::BTreeSet<String> =
        f.list.iter().map(|x| x.encoding.clone()).collect();
    assert!(
        forms.contains("raw") && forms.iter().any(|e| e.starts_with("base64")),
        "{forms:?}"
    );
    assert!(f.list.iter().all(|x| x.class == Class::Secret));
}
