//! The diary stand-in of the sensitive-data tests: contract, table of tools, the program behind them and the releases.
//! Test helpers; no pattern of their own.
#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use capnp_compiler::cligen::rt::{Input, Outcome, Output, Value as V};
use capnp_compiler::mcpgen::rt::{Danger, ToolSpec};
use fs_mcp::canary::{CanaryTable, Class};
use fs_mcp::serve::calls::{Call, Calls};
use fs_mcp::serve::{Connection, Grants, Recipient, Server, ServerInfo, Started};
use futures::executor::LocalPool;
use serde_json::json;

use super::serve::{meta, request};
use fs_mcp::{
    Blocklist, Catalog, Contract, Field, Generator, Interface, Mark, Method, RiskClass, RiskSource,
    StructDef, Target, ToolGenerator, TypeName, TypeRef,
};

const FILE: &str = "diary";

pub fn f(name: &str, ty: TypeRef) -> Field {
    Field {
        name: name.into(),
        doc: String::new(),
        ty,
        in_union: false,
        secret: false,
        sensitive: false,
    }
}
pub fn m(name: &str, results: Vec<Field>) -> Method {
    Method {
        name: name.into(),
        doc: format!("{name} of the diary"),
        params: vec![],
        results,
        streaming: false,
        unclear: false,
    }
}
pub fn st(n: &str) -> TypeRef {
    TypeRef::Struct(TypeName {
        file: FILE.into(),
        name: n.into(),
    })
}

pub fn contract() -> Vec<Contract> {
    vec![Contract {
        file: FILE.into(),
        interfaces: vec![Interface {
            name: "Diary".into(),
            doc: String::new(),
            methods: vec![
                m(
                    "read",
                    vec![
                        f("entries", TypeRef::List(Box::new(st("Entry")))),
                        f("title", TypeRef::Text),
                    ],
                ),
                m("mood", vec![f("mood", st("Mood"))]),
                m("export", vec![f("ics", TypeRef::Text)]),
                m("plain", vec![f("title", TypeRef::Text)]),
                m("token", vec![f("token", TypeRef::Text)]),
            ],
        }],
        structs: vec![
            StructDef {
                name: "Entry".into(),
                doc: String::new(),
                fields: vec![f("day", TypeRef::Text), f("line", TypeRef::Text)],
            },
            StructDef {
                name: "Mood".into(),
                doc: String::new(),
                fields: vec![f("score", TypeRef::Text)],
            },
        ],
        enums: vec![],
    }]
}

pub struct Read;
impl RiskSource for Read {
    fn mark(&self, t: &Target) -> Option<Mark> {
        t.as_str().starts_with("diary/").then_some(Mark {
            class: RiskClass::Read,
            capability: None,
            resource: false,
            reason: None,
        })
    }
    fn release(&self, _: &Target) -> Option<String> {
        None
    }
    fn format(&self, _: &Target) -> Option<String> {
        None
    }
    fn targets(&self) -> Vec<Target> {
        Vec::new()
    }
}

pub fn catalog() -> Catalog {
    Generator::new(&Read, Blocklist::standard())
        .generate(&contract())
        .expect("generation works")
}

pub fn spec(method: &str, output: &str) -> ToolSpec {
    let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
    ToolSpec {
        name: leak(format!("diary.{method}")),
        title: leak(method.to_owned()),
        description: leak(format!("{method} of the diary")),
        key: leak(format!("diary/Diary.{method}")),
        member: None,
        words: Box::leak(vec![leak(method.to_owned())].into_boxed_slice()),
        input_schema: r#"{"type":"object","additionalProperties":false}"#,
        output_schema: leak(output.to_owned()),
        meta: r#"{"net.freesynergy/mcp":{"actor":"ai","risk":"read"}}"#,
        risk: Some(Danger::Read),
        free_content: false,
    }
}

pub fn table() -> &'static [ToolSpec] {
    let t = vec![
        spec(
            "read",
            r##"{"type":"object","properties":{"entries":{"type":"array","items":{"$ref":"#/$defs/Entry"}},"title":{"type":"string"}},"$defs":{"Entry":{"type":"object","properties":{"day":{"type":"string"},"line":{"type":"string","x-fs-sensitive":true}}}}}"##,
        ),
        spec(
            "mood",
            r##"{"type":"object","properties":{"mood":{"$ref":"#/$defs/Mood","x-fs-sensitive":true}},"$defs":{"Mood":{"type":"object","properties":{"score":{"type":"string"}}}}}"##,
        ),
        spec(
            "export",
            r#"{"type":"object","properties":{"ics":{"type":"string","x-fs-sensitive":true}}}"#,
        ),
        spec(
            "plain",
            r#"{"type":"object","properties":{"title":{"type":"string"}}}"#,
        ),
        spec(
            "token",
            r#"{"type":"object","properties":{"token":{"type":"string"}}}"#,
        ),
    ];
    Box::leak(t.into_boxed_slice())
}

/// The diary: the sensitive places hold the marker, everything else a plain word.
pub struct DiaryCalls {
    pub sens: String,
    pub secret: String,
}

impl Calls for DiaryCalls {
    fn call<'a>(
        &'a self,
        tool: &ToolSpec,
        _who: &Connection,
        _input: &'a mut dyn Input,
        out: &'a mut dyn Output,
    ) -> Option<Call<'a>> {
        let name = tool.name;
        Some(Box::pin(async move {
            out.begin_struct("", "");
            match name {
                "diary.read" => {
                    out.begin_list("entries", "");
                    out.begin_struct("", "");
                    out.field("day", V::Text("monday"));
                    out.field("line", V::Text(&self.sens));
                    out.end();
                    out.end();
                    out.field("title", V::Text("my diary"));
                }
                "diary.mood" => {
                    out.begin_struct("mood", "");
                    out.field("score", V::Text(&self.sens));
                    out.end();
                }
                "diary.export" => out.field("ics", V::Text(&self.sens)),
                "diary.plain" => out.field("title", V::Text("my diary")),
                "diary.token" => out.field("token", V::Text(&self.secret)),
                _ => {}
            }
            out.end();
            Ok(Outcome::none())
        }))
    }
}

/// The releases of the program: a set of recipients, shared so that a test can revoke.
#[derive(Clone, Default)]
pub struct Released(pub Rc<RefCell<HashSet<String>>>);
impl Grants for Released {
    fn allows(&self, who: &Recipient, _tool: &str, _path: &str) -> bool {
        self.0.borrow().contains(who.id())
    }
}
/// Everybody is released: the answer of a program that has no say about WHO (the mutant of "release for all").
pub struct Everybody;
impl Grants for Everybody {
    fn allows(&self, _: &Recipient, _: &str, _: &str) -> bool {
        true
    }
}

pub struct World {
    pub table: CanaryTable,
    pub sens: String,
    pub secret: String,
}

pub fn world() -> World {
    let mut table = CanaryTable::new();
    let sens = table.plant_as("diary.Entry.line", Class::Sensitive);
    let secret = table.plant("diary.Diary.token");
    World {
        table,
        sens,
        secret,
    }
}

pub fn server(w: &World, connection: Connection, grants: Box<dyn Grants>) -> Server<DiaryCalls> {
    Server::new(
        DiaryCalls {
            sens: w.sens.clone(),
            secret: w.secret.clone(),
        },
        table(),
        &catalog(),
        &Started,
        ServerInfo {
            name: "diary".into(),
            version: "0".into(),
        },
    )
    .expect("starts")
    .with_access(connection, grants)
}

pub fn ask(s: &Server<DiaryCalls>, tool: &str, client: Option<&str>) -> String {
    let line = request(
        1,
        "tools/call",
        json!({"_meta": meta(client), "name": tool, "arguments": {}}),
    );
    LocalPool::new()
        .run_until(s.handle_line(&line))
        .unwrap_or_default()
}
