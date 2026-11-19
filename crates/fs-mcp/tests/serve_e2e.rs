//! End to end with the real code: every tool of the real table, called through the server, against a program that answers with
//! real capnp messages of every result type (default data and a few list elements). The generated command code walks them
//! into `JsonOutput`, which refuses a field that the contract does not declare and a place whose schema it does not know.
//! A result that fails here would be refused in production too; one that passes fits its `outputSchema`. Pattern: none (tests).

mod support;

use fs_mcp::serve::{Server, Started, TasksCalls};
use fs_mcp::tasks_mcp;
use jsonschema::Validator;
use serde_json::json;
use support::fakeall::client;
use support::serve::{ask, catalog, info, meta, request};

fn server() -> Server<TasksCalls> {
    Server::new(
        TasksCalls(client()),
        tasks_mcp::TOOLS,
        &catalog(),
        &Started,
        info(),
    )
    .expect("starts")
}

#[test]
fn every_tool_of_the_real_table_gives_a_result_that_the_server_shows_and_its_schema_accepts() {
    let s = server();
    let listed = ask(&s, &request(1, "tools/list", json!({"_meta": meta(None)})));
    let tools = listed["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 20);
    let mut shown = 0;
    for t in tools {
        let name = t["name"].as_str().unwrap();
        let line = request(
            2,
            "tools/call",
            json!({"_meta": meta(None), "name": name, "arguments": support::inputs::variants(&t["inputSchema"].to_string())[0]}),
        );
        let r = ask(&s, &line)["result"].clone();
        assert_eq!(r["isError"], false, "{name}: {r}");
        let sc = &r["structuredContent"];
        let v = Validator::new(&t["outputSchema"]).expect("schema");
        assert!(
            v.is_valid(sc),
            "{name}: {sc} {:?}",
            v.iter_errors(sc).map(|e| e.to_string()).collect::<Vec<_>>()
        );
        shown += 1;
    }
    assert_eq!(shown, 20);
}
