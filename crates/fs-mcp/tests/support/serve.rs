//! The server under test: a fake program for `tasks.capnp` (in-process capnp-rpc), the lines to send, and what comes back.
//! Test helpers; no pattern of their own.
#![allow(dead_code, clippy::needless_pass_by_value)] // the helpers take their JSON by value so a call site can write `json!(..)` inline

use std::cell::RefCell;
use std::future::{Future, ready};
use std::rc::Rc;

use capnp_compiler::mcpgen::rt::Switch;
use fs_interfaces::tasks_capnp::{items, tasks};
use fs_mcp::serve::{Server, ServerInfo, Started, TasksCalls};
use fs_mcp::{Blocklist, Catalog, Generator, ToolGenerator, tasks_mcp};
use futures::executor::LocalPool;
use serde_json::{Value, json};

use super::real::{contracts, marks};

/// What the fake program was asked, by key.
pub type Log = Rc<RefCell<Vec<String>>>;

struct FakeItems(Log);
struct FakeTasks(Log);

impl items::Server for FakeItems {
    fn get(
        self: capnp::capability::Rc<Self>,
        params: items::GetParams,
        mut results: items::GetResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        let r = (|| {
            let id = params.get()?.get_id()?;
            self.0
                .borrow_mut()
                .push(format!("get {}:{}", id.get_hi(), id.get_lo()));
            let mut out = results.get();
            out.set_found(true);
            let mut task = out.init_task();
            task.set_jscalendar("{\"@type\":\"Task\"}");
            task.set_revision(u64::MAX);
            let mut env = task.init_envelope();
            env.set_name("a task");
            let mut id = env.reborrow().init_id();
            id.set_hi(0x0123_4567_89ab_cdef);
            id.set_lo(u64::MAX - 1);
            env.reborrow().init_published().set_unix_millis(i64::MIN);
            let mut to = env.init_to(2);
            to.set(0, "https://example.org/a");
            to.set(1, "https://example.org/b");
            Ok(())
        })();
        ready(r)
    }

    fn readiness(
        self: capnp::capability::Rc<Self>,
        _: items::ReadinessParams,
        mut results: items::ReadinessResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        let mut report = results.get().init_report();
        report.set_readiness(fs_interfaces::tasks_capnp::Readiness::DependencyCancelled);
        report.init_by(1).set(0, "u1");
        ready(Ok(()))
    }

    fn delete(
        self: capnp::capability::Rc<Self>,
        _: items::DeleteParams,
        _: items::DeleteResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        self.0.borrow_mut().push("delete".to_owned());
        ready(Err(capnp::Error::failed(
            "SECRET-IN-REMOTE-TEXT".to_owned(),
        )))
    }
}

impl tasks::Server for FakeTasks {
    fn items(
        self: capnp::capability::Rc<Self>,
        _: tasks::ItemsParams,
        mut results: tasks::ItemsResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        results
            .get()
            .set_items(capnp_rpc::new_client(FakeItems(self.0.clone())));
        ready(Ok(()))
    }
}

/// The catalog the screen compares with: the real contracts and marks.
pub fn catalog() -> Catalog {
    Generator::new(&marks(), Blocklist::standard())
        .generate(&contracts())
        .expect("generation works")
}

pub fn info() -> ServerInfo {
    ServerInfo {
        name: "fs-mcp-test".into(),
        version: "0".into(),
    }
}

/// A server over the fake program, and the log of what the program was asked.
pub fn server_with(
    catalog: &Catalog,
    switch: &dyn Switch,
) -> Result<(Server<TasksCalls>, Log), fs_mcp::serve::StartError> {
    let log = Log::default();
    let client: tasks::Client = capnp_rpc::new_client(FakeTasks(log.clone()));
    Server::new(
        TasksCalls(client),
        tasks_mcp::TOOLS,
        catalog,
        switch,
        info(),
    )
    .map(|s| (s, log))
}

pub fn server() -> (Server<TasksCalls>, Log) {
    server_with(&catalog(), &Started).expect("the server starts")
}

/// The `_meta` every request must carry.
pub fn meta(client: Option<&str>) -> Value {
    let mut m = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {},
    });
    if let Some(n) = client {
        m["io.modelcontextprotocol/clientInfo"] = json!({"name": n, "version": "1"});
    }
    m
}

/// A request line.
pub fn request(id: u64, method: &str, params: Value) -> String {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string()
}

/// Send one line, get the answer as JSON.
pub fn ask(server: &Server<TasksCalls>, line: &str) -> Value {
    let mut pool = LocalPool::new();
    let out = pool.run_until(server.handle_line(line)).expect("an answer");
    assert!(!out.contains('\n'), "one line");
    serde_json::from_str(&out).expect("the answer is JSON")
}

/// `tools/call` with the `_meta` of a client.
pub fn call(server: &Server<TasksCalls>, name: &str, arguments: Value) -> Value {
    ask(
        server,
        &request(
            1,
            "tools/call",
            json!({"_meta": meta(Some("test")), "name": name, "arguments": arguments}),
        ),
    )
}

pub fn uuid() -> Value {
    json!({"hi": "81985529216486895", "lo": "18446744073709551614"})
}
