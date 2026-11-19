//! `tools/list` and `tools/call`.
//!
//! Pattern: Facade over the stages of a call — check the name against the offered tools, check the arguments against the
//! `inputSchema` ([`super::fit`]), run the command code ([`super::calls`]), show the result ([`super::output`]). A tool that
//! the screen does not offer ([`crate::Surface`]) does not exist here: unknown, `-32602`. Arguments that do not fit, an
//! error of the other side and a result that cannot be shown are results with `isError: true` (a tool execution error),
//! never a protocol error, and never carry a field of the result.

use capnp_compiler::cligen::rt::{CallError, ErrorKind};
use capnp_compiler::mcpgen::rt::{self, ToolSpec};
use serde_json::{Value, json};

use super::calls::Calls;
use super::fit::fits;
use super::input::JsonInput;
use super::lifecycle::{Meta, ServerInfo, result_meta};
use super::output::JsonOutput;
use super::rpc::{INVALID_PARAMS, RpcError};

/// A tool that may be listed and called, with its schemas read once.
pub struct Offered {
    /// The table entry.
    pub spec: &'static ToolSpec,
    input: Value,
    output: Value,
}

impl Offered {
    /// Read the schemas of the entry.
    ///
    /// # Errors
    /// A schema of the table is not JSON (the generator writes JSON, so this is a bug of the table).
    pub fn new(spec: &'static ToolSpec) -> Result<Self, String> {
        let read = |what: &str, text: &str| {
            serde_json::from_str(text).map_err(|e| format!("{}: {what}: {e}", spec.name))
        };
        Ok(Self {
            spec,
            input: read("inputSchema", spec.input_schema)?,
            output: read("outputSchema", spec.output_schema)?,
        })
    }

    /// The tool as MCP names it. `readOnlyHint` is false and `destructiveHint` true for every tool, as in the catalog of
    /// this crate: reading is not harmless, and hints are untrusted anyway; the real class is in `_meta`.
    fn to_json(&self) -> Value {
        let s = self.spec;
        json!({
            "name": s.name,
            "title": s.title,
            "description": s.description,
            "inputSchema": self.input,
            "outputSchema": self.output,
            "annotations": {"readOnlyHint": false, "destructiveHint": true, "openWorldHint": true},
            "_meta": serde_json::from_str::<Value>(s.meta).unwrap_or(Value::Null),
        })
    }
}

/// The result of `tools/list`: the same tools in the same order for every connection.
#[must_use]
pub fn list(offered: &[Offered], info: &ServerInfo, meta: &Meta) -> Value {
    json!({
        "resultType": rt::RESULT_TYPE,
        "tools": offered.iter().map(Offered::to_json).collect::<Vec<_>>(),
        "_meta": result_meta(info, meta),
    })
}

fn text_result(
    info: &ServerInfo,
    meta: &Meta,
    text: &str,
    is_error: bool,
    structured: Option<Value>,
) -> Value {
    let mut r = json!({
        "resultType": rt::RESULT_TYPE,
        "content": [{"type": "text", "text": text}],
        "isError": is_error,
        "_meta": result_meta(info, meta),
    });
    if let Some(s) = structured {
        r["structuredContent"] = s;
    }
    r
}

/// What a failed call says; the text of the other side is not passed on (it may hold what a person wrote).
fn failure(e: &CallError) -> String {
    let at = if e.path.is_empty() {
        String::new()
    } else {
        format!(" (`{}`)", e.path)
    };
    match &e.kind {
        ErrorKind::BadValue(why) => format!("invalid argument{at}: {why}"),
        ErrorKind::OutOfRange(ty) => format!("argument{at} does not fit {ty}"),
        ErrorKind::UnionTwice => format!("two members of one union given{at}"),
        ErrorKind::Rpc(c) => format!("the program answered with an error ({:?})", c.kind),
        ErrorKind::NotInSchema(n) => {
            format!("the result holds a value ({n}) that this version does not know")
        }
        ErrorKind::Unsupported(why) => format!("not supported here: {why}"),
    }
}

/// `tools/call`.
///
/// # Errors
/// `-32602` for a missing or malformed `name` or `arguments` and for a tool that is not offered.
pub async fn call<C: Calls>(
    calls: &C,
    offered: &[Offered],
    params: &Value,
    info: &ServerInfo,
    meta: &Meta,
) -> Result<Value, RpcError> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::new(INVALID_PARAMS, "`name` must be the name of a tool"))?;
    let arguments = match params.get("arguments") {
        None => json!({}),
        Some(a @ Value::Object(_)) => a.clone(),
        Some(_) => {
            return Err(RpcError::new(
                INVALID_PARAMS,
                "`arguments` must be an object",
            ));
        }
    };
    let tool = offered
        .iter()
        .find(|t| t.spec.name == name)
        .ok_or_else(|| RpcError::new(INVALID_PARAMS, format!("Unknown tool: {name}")))?;
    let error = |text: &str| Ok(text_result(info, meta, text, true, None));
    if let Err(why) = fits(&tool.input, &tool.input, &arguments) {
        return error(&format!("invalid arguments: {why}"));
    }
    let mut input = JsonInput::new(arguments);
    let mut output = JsonOutput::new(&tool.output);
    let Some(run) = calls.call(tool.spec, &mut input, &mut output) else {
        return error("this tool has no command in this program");
    };
    let outcome = run.await;
    match outcome {
        Err(e) => error(&failure(&e)),
        Ok(o) if o.errors() > 0 => error("a call of a listener could not be read"),
        Ok(_) => match output.finish() {
            Err(why) => error(&format!("the result cannot be shown: {why}")),
            Ok(v) => {
                let text = serde_json::to_string(&v).unwrap_or_default();
                Ok(text_result(info, meta, &text, false, Some(v)))
            }
        },
    }
}
