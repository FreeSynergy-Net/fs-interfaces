//! The canary test against the tools of this repository: markers in every place where a secret would sit, every tool called
//! the way a client can call it, every answer, error text and log searched for them. Pattern: none (tests).
//! It does not stop at the first leak: it lists ALL of them (`FSCANARY_FEHLERLISTE` names the file) and then fails.
#![cfg(feature = "conformance")]

mod support;

use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

use fs_mcp::canary::{Canary, CanaryTable, Class};
use serde_json::{Value, json};
use support::canary::{Findings, branch, drive, programs, setup, vault_prog};

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
    // the self-test of the log search: a program that writes a secret into its log
    if let Ok(how) = std::env::var("FSCANARY_LEAK") {
        let marker = table
            .borrow()
            .marker("(program error text)")
            .map(str::to_owned)
            .unwrap_or_default();
        match how.as_str() {
            "out" => println!("out: connected with {marker}"),
            "log" => eprintln!("log: failed for {marker}"),
            // a panic with the secret in its message: the default hook writes it to stderr
            "panic" => {
                let _ = std::panic::catch_unwind(|| panic!("failed for {marker}"));
            }
            other => panic!("no such way to leak: {other}"),
        }
    }
}

fn logs_of_a_child(table: &CanaryTable, f: &mut Findings, leak: Option<&str>) {
    let mut cmd = Command::new(std::env::current_exe().expect("the test binary"));
    if let Some(how) = leak {
        cmd.env("FSCANARY_LEAK", how);
    }
    let out = cmd
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
    logs_of_a_child(&canaries, &mut f, None);
    // no failure mode of the program was left out
    let want = [
        "rpc-error-with-a-canary-in-its-text",
        "rpc-error",
        "unreadable",
        "extra",
        "empty",
    ];
    assert_eq!(
        f.modes.iter().map(String::as_str).collect::<Vec<_>>().len(),
        want.len(),
        "{:?}",
        f.modes
    );
    for m in want {
        assert!(f.modes.contains(m), "{m} was run");
    }
    // not empty-handed: the stand-in vault has places for four secrets, and the tasks tools have none that is not released
    for place in [
        "vault.tokens.tokens",
        "vault.credentials.credentials",
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
fn canary_the_driver_finds_a_leak_that_is_there() {
    // a human wrongly released every place of the vault: the screen lets the secrets through, the driver must see them
    let progs = vec![vault_prog(true)];
    let table = setup(&progs);
    let mut f = Findings::default();
    drive(&mut f, &progs[0], &table);
    assert_ne!(progs[0].catalog.tools.len(), 0);
    let paths: std::collections::BTreeSet<&str> = f.list.iter().map(|x| x.path.as_str()).collect();
    for place in [
        "vault.tokens.tokens",
        "vault.credentials.credentials",
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

#[test]
fn canary_the_logs_of_a_child_are_searched_stdout_and_stderr() {
    // a program that leaks into stdout, into a log line (stderr) and into the message of a panic (stderr): each is found
    for (how, place) in [("out", "stdout"), ("log", "stderr"), ("panic", "stderr")] {
        let progs = programs();
        let table = setup(&progs);
        let canaries = CanaryTable::from_canaries(table.borrow().all().to_vec());
        let mut f = Findings::default();
        logs_of_a_child(&canaries, &mut f, Some(how));
        assert!(
            f.list
                .iter()
                .any(|x| x.place.contains(place) && x.path == "(program error text)"),
            "{how}: {:?}",
            f.list
        );
        assert!(
            f.list.iter().all(|x| x.path == "(program error text)"),
            "{how}: only that canary: {:?}",
            f.list
        );
    }
}
