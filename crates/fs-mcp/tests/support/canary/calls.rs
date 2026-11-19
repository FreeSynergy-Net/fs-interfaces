//! The program under test: a store that answers every tool from its output schema, with a canary in every field that a
//! secret would sit in, and a plain value everywhere else.
//! Test helpers; no pattern of their own. One implementation serves every table (tasks, the vault).
#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;

use capnp_compiler::cligen::model::kebab;
use capnp_compiler::cligen::rt::{CallError, ErrorKind, Input, Outcome, Output, Value as V};
use capnp_compiler::mcpgen::rt::ToolSpec;
use fs_mcp::canary::CanaryTable;
use fs_mcp::serve::calls::{Call, Calls};
use fs_mcp::serve::schema::resolve;
use fs_mcp::{doc_looks_secret, name_looks_secret};
use serde_json::Value;

/// How the store behaves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Answers with its data.
    Normal,
    /// Fails; the text of the error holds the given text (a canary, in the tests that look for echoes).
    RpcError(String),
    /// Answers, but one of the values cannot be read.
    Unreadable,
    /// Answers with nothing.
    Empty,
}

/// The store.
#[derive(Clone)]
pub struct StoreCalls {
    pub table: Rc<RefCell<CanaryTable>>,
    /// Places a human released in writing (`common/SupportReport.entries`): nothing is planted there.
    pub released: Vec<String>,
    pub mode: Mode,
    /// Only walk the schemas and plant the canaries, write nothing.
    pub plant: bool,
}

impl StoreCalls {
    pub fn new(table: Rc<RefCell<CanaryTable>>, released: Vec<String>) -> Self {
        Self {
            table,
            released,
            mode: Mode::Normal,
            plant: false,
        }
    }

    pub fn with_mode(&self, mode: Mode) -> Self {
        Self {
            mode,
            ..self.clone()
        }
    }

    /// The paths of the fields of a tool's output where a secret would sit (name or comment looks like one), minus the
    /// places released in writing. Planted in the table.
    pub fn plant_for(&self, tool: &ToolSpec) {
        let me = Self {
            plant: true,
            ..self.clone()
        };
        let root: Value = serde_json::from_str(tool.output_schema).expect("schema");
        me.object(
            &root,
            &root,
            tool.name,
            &mut capnp_compiler::cligen::rt::Discard,
            0,
        );
    }

    fn released_here(&self, def: &str, prop: &str) -> bool {
        // `common.SupportReport` + `entries` is the place `common/SupportReport.entries`
        let place = def.replacen('.', "/", 1) + "." + prop;
        self.released.contains(&place)
    }

    fn object(&self, root: &Value, schema: &Value, def: &str, out: &mut dyn Output, depth: u32) {
        let s = resolve(root, schema);
        let Some(props) = s.get("properties").and_then(Value::as_object) else {
            return;
        };
        for (k, ps) in props {
            if depth > 4 {
                return;
            }
            let own = ps
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|r| r.strip_prefix("#/$defs/"));
            let pr = resolve(root, ps);
            let desc = ps
                .get("description")
                .or_else(|| pr.get("description"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let path = format!("{def}.{k}");
            // a place where a secret would sit: a name or comment that says so, or content the screen cannot judge (bytes, or a
            // field named json, config, proof, payload, blob, data)
            let lower = k.to_lowercase();
            let free = pr.get("contentEncoding").is_some()
                || ["json", "config", "proof", "payload", "blob", "data"]
                    .iter()
                    .any(|w| lower.contains(w));
            let secret_place = (name_looks_secret(k) || doc_looks_secret(desc) || free)
                && !self.released_here(def, k);
            let leaf = pr.get("type").and_then(Value::as_str) == Some("string")
                && pr.get("enum").is_none()
                && pr.get("x-fs-int").is_none();
            if secret_place && leaf && self.plant {
                self.table.borrow_mut().plant(&path);
            }
            self.value(root, pr, k, own.unwrap_or(&path), secret_place, out, depth);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn value(
        &self,
        root: &Value,
        pr: &Value,
        name: &str,
        path: &str,
        secret: bool,
        out: &mut dyn Output,
        depth: u32,
    ) {
        let marker = if secret {
            self.table.borrow().marker(path).map(str::to_owned)
        } else {
            None
        };
        let text = marker.clone().unwrap_or_else(|| "plain".to_owned());
        if let Some(names) = pr.get("enum").and_then(Value::as_array) {
            if let Some(n) = names.first().and_then(Value::as_str) {
                out.field(name, V::Enum(Box::leak(kebab(n).into_boxed_str())));
            }
            return;
        }
        if pr.get("anyOf").is_some() {
            out.field(name, V::Float(0.5));
            return;
        }
        match pr.get("type").and_then(Value::as_str) {
            Some("object") => {
                out.begin_struct(name, "");
                self.object(root, pr, path, out, depth + 1);
                out.end();
            }
            Some("array") => {
                out.begin_list(name, "");
                if let Some(items) = pr.get("items") {
                    let ir = resolve(root, items);
                    if ir.get("type").and_then(Value::as_str) == Some("object") {
                        let idef = items
                            .get("$ref")
                            .and_then(Value::as_str)
                            .and_then(|r| r.strip_prefix("#/$defs/"))
                            .unwrap_or(path);
                        out.begin_struct("", "");
                        self.object(root, ir, idef, out, depth + 1);
                        out.end();
                    } else {
                        self.value(root, ir, "", path, secret, out, depth + 1);
                    }
                }
                out.end();
            }
            Some("string") if pr.get("x-fs-int").is_some() => {
                let signed = pr["x-fs-int"]
                    .get("signed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                out.field(name, if signed { V::Int(-1) } else { V::Uint(1) });
            }
            Some("string") if pr.get("contentEncoding").is_some() => {
                out.field(name, V::Data(text.as_bytes()));
            }
            Some("string") => out.field(name, V::Text(&text)),
            Some("integer") => out.field(name, V::Uint(1)),
            Some("number") => out.field(name, V::Float(0.5)),
            Some("boolean") => out.field(name, V::Bool(true)),
            _ => {}
        }
    }
}

impl Calls for StoreCalls {
    fn call<'a>(
        &'a self,
        tool: &ToolSpec,
        _input: &'a mut dyn Input,
        output: &'a mut dyn Output,
    ) -> Option<Call<'a>> {
        let (schema, name) = (tool.output_schema, tool.name);
        Some(Box::pin(async move {
            match &self.mode {
                Mode::RpcError(text) => {
                    return Err(CallError::at(
                        "",
                        ErrorKind::Rpc(capnp::Error::failed(text.clone())),
                    ));
                }
                Mode::Empty => return Ok(Outcome::none()),
                Mode::Unreadable => {
                    output.begin_struct("", "");
                    output.field("x", V::Unreadable("any pointer"));
                    output.field("y", V::UnknownEnum(9));
                    output.end();
                    return Ok(Outcome::none());
                }
                Mode::Normal => {}
            }
            let root: Value = serde_json::from_str(schema).expect("the schema is JSON");
            output.begin_struct("", "");
            self.object(&root, &root, name, output, 0);
            output.end();
            Ok(Outcome::none())
        }))
    }
}

/// The key of the store, as the program keeps it: its `Debug` says `<redacted>` (like `EncryptionKey` in `fs-objects`).
pub struct StoreKey {
    pub key: String,
}

impl std::fmt::Debug for StoreKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = &self.key;
        f.write_str("StoreKey(<redacted>)")
    }
}
