//! The driver: every tool, with arguments made from its schema, the paths of failure, the export, and what a client can ask
//! that is not a tool; every answer is searched for the canaries.
//! Test helpers; no pattern of their own.
#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;

use capnp_compiler::mcpgen::rt::ToolSpec;
use fs_mcp::canary::CanaryTable;
use fs_mcp::serve::{Server, ServerInfo, Started};
use fs_mcp::{Catalog, tasks_mcp};
use futures::executor::LocalPool;
use serde_json::{Value, json};

use super::calls::{Mode, StoreCalls};
use super::find::Findings;
use super::{inputs, vault};
use crate::support::real::{contracts, marks};
use crate::support::serve::{meta, request};

/// A program: its table of tools, the catalog the screen compares with, and the places released in writing.
pub struct Prog {
    pub name: &'static str,
    pub table: &'static [ToolSpec],
    pub catalog: Catalog,
    pub released: Vec<String>,
}

/// The real table of `tasks.capnp` with the real contracts and marks.
pub fn tasks() -> Prog {
    use fs_mcp::{Blocklist, Generator, RiskSource, ToolGenerator};
    let m = marks();
    let catalog = Generator::new(&m, Blocklist::standard())
        .generate(&contracts())
        .expect("generation works");
    // places a human released in writing: a `[release]` of a name or of free content, and the `paths` of a `[[tool_release]]`
    let targets = m.targets();
    let mut released: Vec<String> = targets
        .iter()
        .filter(|t| m.release(t).is_some())
        .map(ToString::to_string)
        .collect();
    released.extend(
        targets
            .iter()
            .filter_map(|t| m.tool_release(t))
            .flat_map(|r| r.paths),
    );
    released.sort();
    released.dedup();
    Prog {
        name: "tasks",
        table: tasks_mcp::TOOLS,
        catalog,
        released,
    }
}

/// The stand-in for the vault; with `release_all` the screen is wrongly told that everything is fine.
pub fn vault_prog(release_all: bool) -> Prog {
    Prog {
        name: "vault",
        table: vault::table(),
        catalog: vault::catalog(release_all),
        released: Vec::new(),
    }
}

pub fn server(p: &Prog, calls: StoreCalls) -> Server<StoreCalls> {
    Server::new(
        calls,
        p.table,
        &p.catalog,
        &Started,
        ServerInfo {
            name: format!("canary-{}", p.name),
            version: "0".into(),
        },
    )
    .expect("the server starts")
}

/// Plant a canary in every place of every tool where a secret would sit.
pub fn plant(p: &Prog, table: &Rc<RefCell<CanaryTable>>) {
    let calls = StoreCalls::new(table.clone(), p.released.clone());
    for t in p.table {
        calls.plant_for(t);
    }
}

/// Ask: the line, the answer as text (`""` if there is none).
pub fn ask(s: &Server<StoreCalls>, line: &str) -> String {
    LocalPool::new()
        .run_until(s.handle_line(line))
        .unwrap_or_default()
}

fn call_line(name: &str, args: &Value) -> String {
    request(
        1,
        "tools/call",
        json!({"_meta": meta(Some("canary")), "name": name, "arguments": args}),
    )
}

fn snapshot(table: &Rc<RefCell<CanaryTable>>) -> CanaryTable {
    CanaryTable::from_canaries(table.borrow().all().to_vec())
}

/// Everything a client can do against one program.
#[allow(clippy::too_many_lines)] // one driver: the list of what a client can do reads best in one place
pub fn drive(f: &mut Findings, p: &Prog, table: &Rc<RefCell<CanaryTable>>) {
    let calls = StoreCalls::new(table.clone(), p.released.clone());
    let normal = server(p, calls.clone());
    let canaries = snapshot(table);
    let check = |f: &mut Findings, place: String, line: &str, s: &Server<StoreCalls>| {
        f.calls += 1;
        let text = ask(s, line);
        f.check(&format!("{}: {place}", p.name), &text, &canaries);
    };
    // what a client can ask that is no tool
    for (what, line) in [
        (
            "server/discover",
            request(1, "server/discover", json!({"_meta": meta(Some("canary"))})),
        ),
        ("ping", request(1, "ping", json!({"_meta": meta(None)}))),
        (
            "tools/list",
            request(1, "tools/list", json!({"_meta": meta(Some("canary"))})),
        ),
    ] {
        check(f, what.to_owned(), &line, &normal);
    }
    let offered = normal.tool_names();
    f.note(format!(
        "{}: {} tools in the table, {} offered",
        p.name,
        p.table.len(),
        offered.len()
    ));
    // every tool of the TABLE (also those the screen keeps out: they must be unknown), every valid and every wrong argument
    for t in p.table {
        for (i, args) in inputs::variants(t.input_schema).iter().enumerate() {
            check(
                f,
                format!("tools/call {} variant {i}", t.name),
                &call_line(t.name, args),
                &normal,
            );
        }
        for (i, args) in inputs::wrong(t.input_schema).iter().enumerate() {
            check(
                f,
                format!("tools/call {} wrong arguments {i}", t.name),
                &call_line(t.name, args),
                &normal,
            );
        }
        // the tool of another name that is nearly this one, and the same in other letter case
        for name in [
            format!("{}x", t.name),
            t.name.to_uppercase(),
            t.name.replace('.', "/"),
        ] {
            check(
                f,
                format!("tools/call unknown {name}"),
                &call_line(&name, &json!({})),
                &normal,
            );
        }
        // the program fails, answers with nothing, answers with a value that cannot be read
        let inner = table
            .borrow()
            .marker("(program error text)")
            .map(str::to_owned);
        let key = super::calls::StoreKey {
            key: table
                .borrow()
                .marker("store.EncryptionKey.key")
                .unwrap_or_default()
                .to_owned(),
        };
        let failing = [
            Mode::RpcError(format!(
                "store failed for key {}",
                inner.unwrap_or_default()
            )),
            Mode::RpcError(format!("could not decrypt with {key:?}")),
            Mode::Unreadable,
            Mode::Extra,
            Mode::Empty,
        ];
        for mode in failing {
            let kind = match &mode {
                Mode::RpcError(text)
                    if table
                        .borrow()
                        .all()
                        .iter()
                        .any(|c| text.contains(&c.marker)) =>
                {
                    "rpc-error-with-a-canary-in-its-text".to_owned()
                }
                Mode::RpcError(_) => "rpc-error".to_owned(),
                other => format!("{other:?}").to_lowercase(),
            };
            f.modes.insert(kind);
            let s = server(p, calls.with_mode(mode.clone()));
            check(
                f,
                format!("tools/call {} program {mode:?}", t.name),
                &call_line(t.name, &inputs::variants(t.input_schema)[0]),
                &s,
            );
        }
    }
    // errors of the protocol: broken lines, methods that do not exist, no `_meta`, another version, no name
    let lines: Vec<String> = vec![
        "{".into(), "}".into(), "null".into(), "[1]".into(), String::new(), "\u{0}".into(), "\"x\"".into(), "{\"jsonrpc\":\"2.0\"}".into(),
        "{\"jsonrpc\":\"2.0\",\"id\":1}".into(), "{\"jsonrpc\":\"2.0\",\"id\":[1],\"method\":\"ping\"}".into(),
        request(1, "initialize", json!({})), request(1, "tools/nothing", json!({"_meta": meta(None)})), request(1, "tools/list", json!({})),
        request(1, "tools/call", json!({"_meta": meta(None)})), request(1, "tools/call", json!({"_meta": meta(None), "name": 5})),
        request(1, "tools/call", json!({"_meta": meta(None), "name": "x", "arguments": []})),
        request(1, "tools/list", json!({"_meta": {"io.modelcontextprotocol/protocolVersion": "1900-01-01", "io.modelcontextprotocol/clientCapabilities": {}}})),
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"\\ud800\"}}".into(),
    ];
    for l in &lines {
        check(f, format!("line {l:?}"), l, &normal);
    }
}
