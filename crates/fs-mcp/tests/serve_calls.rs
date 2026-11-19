//! What a call returns and what it never returns: the structured result against its schema, the errors of the protocol
//! and of the tool, and what never reaches the fake program. Pattern: none (tests).

mod support;

use jsonschema::Validator;
use serde_json::{Value, json};
use support::serve::{ask, call, meta, request, server, uuid};

fn code(v: &Value) -> i64 {
    v["error"]["code"].as_i64().expect("an error")
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

fn uuid_back() -> Value {
    json!({"hi": "81985529216486895", "lo": "18446744073709551614"})
}

/// A program whose calls end in the way the real tasks table cannot show: a watch with unreadable calls, and a call that
/// writes no result.
struct Odd(bool);

impl fs_mcp::serve::Calls for Odd {
    fn call<'a>(
        &'a self,
        _: &capnp_compiler::mcpgen::rt::ToolSpec,
        _: &'a mut dyn capnp_compiler::cligen::rt::Input,
        _: &'a mut dyn capnp_compiler::cligen::rt::Output,
    ) -> Option<fs_mcp::serve::calls::Call<'a>> {
        let counted = self.0;
        Some(Box::pin(async move {
            let errors = if counted {
                vec![std::rc::Rc::new(std::cell::Cell::new(2))]
            } else {
                vec![]
            };
            Ok(capnp_compiler::cligen::rt::Outcome::new(
                capnp_compiler::cligen::rt::Held::none(),
                errors,
            ))
        }))
    }
}

#[test]
fn unreadable_calls_of_a_watch_and_an_empty_result_are_tool_errors() {
    for counted in [true, false] {
        let s = fs_mcp::serve::Server::new(
            Odd(counted),
            fs_mcp::tasks_mcp::TOOLS,
            &support::serve::catalog(),
            &fs_mcp::serve::Started,
            support::serve::info(),
        )
        .unwrap();
        let line = request(
            1,
            "tools/call",
            json!({"_meta": meta(None), "name": "tasks.capabilities", "arguments": {}}),
        );
        let r = futures::executor::LocalPool::new()
            .run_until(s.handle_line(&line))
            .unwrap();
        let r: Value = serde_json::from_str(&r).unwrap();
        assert_eq!(r["result"]["isError"], true, "counted {counted}");
        assert!(r["result"].get("structuredContent").is_none());
    }
}
