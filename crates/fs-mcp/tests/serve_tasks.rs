//! The server against `tasks.capnp` and a fake program: the transcript of the revision 2026-07-28 (stateless: no
//! `initialize`, `_meta` on every request), what a call returns and what it never returns. Pattern: none (tests).

mod support;

use capnp_compiler::mcpgen::rt::{Never, Off};
use fs_mcp::serve::{StartError, Started};
use jsonschema::Validator;
use serde_json::{Value, json};
use support::serve::{ask, call, catalog, meta, request, server, server_with, uuid};

fn code(v: &Value) -> i64 {
    v["error"]["code"].as_i64().expect("an error")
}

#[test]
fn a_request_without_meta_or_protocol_version_is_invalid_params() {
    let (s, _) = server();
    for params in [
        json!({}),
        json!({"_meta": {}}),
        json!({"_meta": {"io.modelcontextprotocol/protocolVersion": "2026-07-28"}}),
        json!({"_meta": {"io.modelcontextprotocol/clientCapabilities": {}}}),
        json!({"_meta": {"io.modelcontextprotocol/protocolVersion": 3, "io.modelcontextprotocol/clientCapabilities": {}}}),
    ] {
        for method in ["server/discover", "tools/list", "ping", "tools/call"] {
            let a = ask(&s, &request(1, method, params.clone()));
            assert_eq!(code(&a), -32602, "{method} {params}");
        }
    }
    let a = ask(&s, &request(1, "tools/list", json!({})));
    assert_eq!(
        a["error"]["data"]["missing"].as_array().map(Vec::len),
        Some(2)
    );
}

#[test]
fn another_version_is_32022_with_the_supported_list() {
    let (s, _) = server();
    let mut m = meta(None);
    m["io.modelcontextprotocol/protocolVersion"] = json!("2025-11-25");
    let a = ask(&s, &request(7, "server/discover", json!({"_meta": m})));
    assert_eq!(code(&a), -32022);
    assert_eq!(a["error"]["data"]["supported"], json!(["2026-07-28"]));
    assert_eq!(a["error"]["data"]["requested"], "2025-11-25");
    assert_eq!(a["id"], 7);
}

#[test]
fn initialize_is_an_error_that_names_the_revision_and_makes_no_session() {
    let (s, _) = server();
    let a = ask(
        &s,
        &request(
            1,
            "initialize",
            json!({"protocolVersion": "2025-11-25", "capabilities": {}, "clientInfo": {"name": "x"}}),
        ),
    );
    assert!(
        a["error"]["message"]
            .as_str()
            .unwrap()
            .contains("2026-07-28")
    );
    // nothing was remembered: the next request without `_meta` is still malformed
    assert_eq!(code(&ask(&s, &request(2, "tools/list", json!({})))), -32602);
}

#[test]
fn discover_names_the_version_the_tools_and_the_server() {
    let (s, _) = server();
    let a = ask(
        &s,
        &request(1, "server/discover", json!({"_meta": meta(Some("x"))})),
    );
    let r = &a["result"];
    assert_eq!(r["resultType"], "complete");
    assert_eq!(r["supportedVersions"], json!(["2026-07-28"]));
    assert!(r["capabilities"]["tools"].is_object());
    assert_eq!(
        r["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "fs-mcp-test"
    );
}

#[test]
fn the_label_of_the_actor_is_per_request_always_ai_and_decides_nothing() {
    let (s, _) = server();
    let mut labels = Vec::new();
    for name in ["x", "human:alice", "\u{202e}evil", ""] {
        let a = ask(&s, &request(1, "ping", json!({"_meta": meta(Some(name))})));
        let ours = &a["result"]["_meta"]["net.freesynergy/mcp"];
        assert_eq!(ours["ai"], true);
        labels.push(ours["actor"].as_str().unwrap().to_owned());
    }
    assert_eq!(labels, ["ai:x", "ai:human_alice", "ai:_evil", "ai:unknown"]);
    // the same rights for every label: the same tools
    let l = |n: &str| {
        ask(
            &s,
            &request(1, "tools/list", json!({"_meta": meta(Some(n))})),
        )["result"]["tools"]
            .clone()
    };
    assert_eq!(l("x"), l("human:alice"));
    // a call carries the label too
    let r = call(&s, "tasks.capabilities", json!({}));
    assert_eq!(
        r["result"]["_meta"]["net.freesynergy/mcp"]["actor"],
        "ai:test"
    );
}

#[test]
fn the_list_is_byte_equal_every_time_and_is_what_the_screen_offers() {
    let (s, _) = server();
    let line = request(1, "tools/list", json!({"_meta": meta(Some("x"))}));
    let mut pool = futures::executor::LocalPool::new();
    let a = pool.run_until(s.handle_line(&line)).unwrap();
    let b = pool.run_until(s.handle_line(&line)).unwrap();
    assert_eq!(a, b);
    let v: Value = serde_json::from_str(&a).unwrap();
    assert_eq!(v["result"]["resultType"], "complete");
    let tools = v["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 19);
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, s.tool_names());
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 19, "names are unique");
    for t in tools {
        assert_eq!(t["inputSchema"]["type"], "object", "{}", t["name"]);
        assert_eq!(t["_meta"]["net.freesynergy/mcp"]["actor"], "ai");
        assert_eq!(t["annotations"]["readOnlyHint"], false);
    }
}

#[test]
fn get_returns_a_structured_result_that_fits_its_output_schema_plus_a_text_block() {
    let (s, log) = server();
    let r = call(&s, "tasks.items.get", json!({"id": uuid()}))["result"].clone();
    assert_eq!(r["resultType"], "complete");
    assert_eq!(r["isError"], false);
    let sc = &r["structuredContent"];
    assert_eq!(sc["found"], true);
    // 64-bit integers are strings, the list is a list, the text is the text
    assert_eq!(sc["task"]["revision"], "18446744073709551615");
    assert_eq!(sc["task"]["envelope"]["id"], uuid_back());
    assert_eq!(
        sc["task"]["envelope"]["published"]["unixMillis"],
        "-9223372036854775808"
    );
    assert_eq!(
        sc["task"]["envelope"]["to"],
        json!(["https://example.org/a", "https://example.org/b"])
    );
    let text: Value = serde_json::from_str(r["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        &text, sc,
        "the text block is the serialized structured content"
    );
    // it satisfies the output schema of the tool (the jsonschema crate, the real validator)
    let tools = ask(&s, &request(1, "tools/list", json!({"_meta": meta(None)})));
    let tool = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "tasks.items.get")
        .unwrap();
    let v = Validator::new(&tool["outputSchema"]).expect("the output schema is a schema");
    assert!(
        v.is_valid(sc),
        "{:?}",
        v.iter_errors(sc).map(|e| e.to_string()).collect::<Vec<_>>()
    );
    assert_eq!(
        log.borrow().as_slice(),
        ["get 81985529216486895:18446744073709551614"]
    );
}

fn uuid_back() -> Value {
    json!({"hi": "81985529216486895", "lo": "18446744073709551614"})
}

#[test]
fn an_unknown_tool_and_a_malformed_call_are_protocol_errors() {
    let (s, log) = server();
    assert_eq!(code(&call(&s, "tasks.nothing", json!({}))), -32602);
    let a = ask(
        &s,
        &request(
            1,
            "tools/call",
            json!({"_meta": meta(None), "arguments": {}}),
        ),
    );
    assert_eq!(code(&a), -32602);
    let a = ask(
        &s,
        &request(
            1,
            "tools/call",
            json!({"_meta": meta(None), "name": "tasks.items.get", "arguments": [1]}),
        ),
    );
    assert_eq!(code(&a), -32602);
    assert_eq!(
        code(&ask(
            &s,
            &request(1, "tools/nothing", json!({"_meta": meta(None)}))
        )),
        -32601
    );
    assert_eq!(log.borrow().len(), 0, "nothing reached the program");
}

#[test]
fn arguments_that_break_the_schema_are_tool_errors_and_never_run() {
    let (s, log) = server();
    for args in [
        json!({}),
        json!({"id": uuid(), "extra": 1}),
        json!({"id": {"hi": 1, "lo": "2"}}),
        json!({"id": {"hi": "1"}}),
        json!({"id": {"hi": "18446744073709551616", "lo": "2"}}),
        json!({"id": {"hi": "x", "lo": "2"}}),
        json!({"id": "nope"}),
    ] {
        let r = call(&s, "tasks.items.get", args.clone())["result"].clone();
        assert_eq!(r["isError"], true, "{args}");
        assert!(r.get("structuredContent").is_none());
    }
    assert_eq!(
        log.borrow().len(),
        0,
        "an argument that does not fit never reaches the program"
    );
}

#[test]
fn an_error_of_the_program_is_a_tool_error_and_its_text_is_not_passed_on() {
    let (s, log) = server();
    let r = call(&s, "tasks.items.delete", json!({"id": uuid()}))["result"].clone();
    assert_eq!(r["isError"], true);
    assert!(!r.to_string().contains("SECRET-IN-REMOTE-TEXT"));
    assert_eq!(log.borrow().as_slice(), ["delete"]);
}

#[test]
fn the_server_does_not_start_while_the_switch_is_off_and_never_for_the_vault() {
    for sw in [&Off as &dyn capnp_compiler::mcpgen::rt::Switch, &Never] {
        assert!(matches!(
            server_with(&catalog(), sw),
            Err(StartError::SwitchOff)
        ));
    }
    // a server that did not start has no way to write a line
    assert!(server_with(&catalog(), &Started).is_ok());
}

#[test]
fn a_tool_the_screen_does_not_offer_is_not_listed_and_not_callable() {
    use fs_mcp::{Blocklist, Generator, ToolGenerator};
    // without the written releases the screen keeps `tasks.support-report` out
    struct NoReleases(fs_mcp::MarkFiles);
    impl fs_mcp::RiskSource for NoReleases {
        fn mark(&self, t: &fs_mcp::Target) -> Option<fs_mcp::Mark> {
            self.0.mark(t)
        }
        fn release(&self, t: &fs_mcp::Target) -> Option<String> {
            self.0.release(t)
        }
        fn format(&self, t: &fs_mcp::Target) -> Option<String> {
            self.0.format(t)
        }
        fn targets(&self) -> Vec<fs_mcp::Target> {
            self.0.targets()
        }
    }
    let m = NoReleases(support::real::marks());
    let cat = Generator::new(&m, Blocklist::standard())
        .generate(&support::real::contracts())
        .unwrap();
    let (s, log) = server_with(&cat, &Started).unwrap();
    assert_eq!(s.tool_names().len(), 18);
    assert!(!s.tool_names().contains(&"tasks.support-report"));
    assert_eq!(code(&call(&s, "tasks.support-report", json!({}))), -32602);
    assert_eq!(log.borrow().len(), 0);
}

#[test]
fn lines_that_are_no_requests_are_answered_or_ignored_as_the_protocol_says() {
    let (s, _) = server();
    let mut pool = futures::executor::LocalPool::new();
    let mut ask_raw = |l: &str| pool.run_until(s.handle_line(l));
    assert_eq!(ask_raw(""), None);
    assert_eq!(ask_raw("   "), None);
    // a notification (no id) is never answered, known method or not
    assert_eq!(
        ask_raw(r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{}}"#),
        None
    );
    assert_eq!(
        ask_raw(r#"{"jsonrpc":"2.0","method":"ping","params":{}}"#),
        None
    );
    let parse: Value = serde_json::from_str(&ask_raw("{not json").unwrap()).unwrap();
    assert_eq!(
        (parse["error"]["code"].as_i64(), &parse["id"]),
        (Some(-32700), &Value::Null)
    );
    for bad in [
        "[]",
        r#"{"jsonrpc":"1.0","id":1,"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":[1],"method":"ping"}"#,
        r#"{"jsonrpc":"2.0","id":1,"method":5}"#,
    ] {
        let v: Value = serde_json::from_str(&ask_raw(bad).unwrap()).unwrap();
        assert_eq!(v["error"]["code"], -32600, "{bad}");
    }
    let v: Value = serde_json::from_str(
        &ask_raw(r#"{"jsonrpc":"2.0","id":"a","method":"ping","params":[]}"#).unwrap(),
    )
    .unwrap();
    assert_eq!(
        (v["error"]["code"].as_i64(), &v["id"]),
        (Some(-32602), &json!("a"))
    );
}

#[test]
fn serve_writes_one_line_per_request_and_only_protocol_on_the_output() {
    let (s, _) = server();
    let input = format!(
        "{}\n\n{}\n{}\n",
        request(1, "ping", json!({"_meta": meta(None)})),
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        request(2, "server/discover", json!({"_meta": meta(None)})),
    );
    let mut out = Vec::new();
    s.serve(
        &mut futures::executor::LocalPool::new(),
        input.as_bytes(),
        &mut out,
    )
    .unwrap();
    let text = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    for (i, l) in lines.iter().enumerate() {
        let v: Value = serde_json::from_str(l).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], i as u64 + 1);
    }
}
