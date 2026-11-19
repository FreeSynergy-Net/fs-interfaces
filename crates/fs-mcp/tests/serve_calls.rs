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
struct Odd(Option<u32>);

impl fs_mcp::serve::Calls for Odd {
    fn call<'a>(
        &'a self,
        _: &capnp_compiler::mcpgen::rt::ToolSpec,
        _: &'a mut dyn capnp_compiler::cligen::rt::Input,
        output: &'a mut dyn capnp_compiler::cligen::rt::Output,
    ) -> Option<fs_mcp::serve::calls::Call<'a>> {
        let counted = self.0;
        Some(Box::pin(async move {
            if counted.is_some() {
                // a result that could be shown: only the count of unreadable calls makes this call an error
                output.begin_struct("", "");
                output.begin_struct("capabilities", "");
                output.end();
                output.end();
            }
            let errors = match counted {
                Some(n) if n > 0 => vec![std::rc::Rc::new(std::cell::Cell::new(n))],
                _ => vec![],
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
    // (unreadable calls counted, whether the call wrote a result, whether the call is an error)
    for (counted, is_error) in [
        (Some(1), true),
        (Some(2), true),
        (None, true),
        (Some(0), false),
    ] {
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
        assert_eq!(r["result"]["isError"], is_error, "counted {counted:?}");
        assert_eq!(r["result"].get("structuredContent").is_some(), !is_error);
    }
}

#[test]
fn a_tool_is_found_by_its_whole_name_only() {
    let (s, log) = server();
    for name in [
        "tasks.items.getX",
        "tasks.items.",
        "tasks.items.ge",
        "tasks.items.get ",
        "TASKS.ITEMS.GET",
        "tasks.items.get.x",
    ] {
        assert_eq!(
            code(&call(&s, name, json!({"id": uuid()}))),
            -32602,
            "{name}"
        );
    }
    assert_eq!(log.borrow().len(), 0);
}

#[test]
fn a_wrong_enum_name_is_refused_by_the_check_before_the_program_is_reached() {
    let (s, log) = server();
    // `createRelation` takes an enumeration; a right name passes the check (the fake program then answers with an error)
    let ok = json!({"id": uuid(), "otherUid": "u", "relation": "dependsOn"});
    let r = call(&s, "tasks.items.create-relation", ok.clone())["result"].clone();
    assert!(
        r["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("the program answered"),
        "{r}"
    );
    for bad in [
        json!("not-a-relation"),
        json!("DependsOn"),
        json!(4),
        json!(null),
    ] {
        let mut args = ok.clone();
        args["relation"] = bad.clone();
        let r = call(&s, "tasks.items.create-relation", args)["result"].clone();
        assert_eq!(r["isError"], true, "{bad}");
        let text = r["content"][0]["text"].as_str().unwrap();
        assert!(!text.contains("the program answered"), "{bad}: {text}");
    }
    assert_eq!(log.borrow().len(), 0);
}

#[test]
fn an_enumerant_in_a_result_is_written_with_its_contract_name_and_fits_the_schema() {
    let (s, _) = server();
    let r = call(&s, "tasks.items.readiness", json!({"id": uuid()}))["result"].clone();
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(
        r["structuredContent"]["report"]["readiness"],
        "dependencyCancelled"
    );
    let tools = ask(&s, &request(1, "tools/list", json!({"_meta": meta(None)})));
    let tool = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "tasks.items.readiness")
        .unwrap();
    let v = Validator::new(&tool["outputSchema"]).unwrap();
    assert!(
        v.is_valid(&r["structuredContent"]),
        "{:?}",
        v.iter_errors(&r["structuredContent"])
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_field_the_contract_does_not_declare_is_refused_with_the_whole_result() {
    use capnp_compiler::cligen::rt::{Output, Value as V};
    let schema = json!({"type":"object","properties":{"a":{"type":"string"},"inner":{"type":"object","properties":{"b":{"type":"string"}}}}});
    let try_with = |f: &dyn Fn(&mut fs_mcp::serve::output::JsonOutput<'_>)| {
        let mut o = fs_mcp::serve::output::JsonOutput::new(&schema);
        o.begin_struct("", "");
        f(&mut o);
        o.end();
        o.finish()
    };
    assert!(try_with(&|o| o.field("a", V::Text("x"))).is_ok());
    for bad in [
        try_with(&|o| o.field("extra", V::Text("x"))),
        try_with(&|o| {
            o.begin_struct("extra", "");
            o.end();
        }),
        try_with(&|o| {
            o.begin_list("extra", "");
            o.end();
        }),
        try_with(&|o| {
            o.begin_struct("inner", "");
            o.field("b", V::Text("x"));
            o.field("c", V::Text("y"));
            o.end();
        }),
    ] {
        assert!(bad.is_err(), "{bad:?}");
    }
    // a result that declares nothing takes nothing
    let empty = json!({"type":"object","additionalProperties":false});
    let mut o = fs_mcp::serve::output::JsonOutput::new(&empty);
    o.begin_struct("", "");
    o.field("x", V::Bool(true));
    o.end();
    assert!(o.finish().is_err());
}
