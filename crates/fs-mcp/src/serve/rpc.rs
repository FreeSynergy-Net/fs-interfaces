//! JSON-RPC 2.0 over lines: reading a request, writing a result or an error.
//!
//! Pattern: none (plain value types and two writers). One message is one line without a newline inside (the stdio
//! transport of the MCP revision 2026-07-28); a notification (no `id`) never gets an answer.

use serde_json::{Map, Value, json};

/// Not valid JSON.
pub const PARSE_ERROR: i64 = -32700;
/// Not a JSON-RPC request.
pub const INVALID_REQUEST: i64 = -32600;
/// No such method.
pub const METHOD_NOT_FOUND: i64 = -32601;
/// A malformed request (a missing `_meta` field, an unknown tool).
pub const INVALID_PARAMS: i64 = -32602;
/// The version the client asks for is not supported.
pub const UNSUPPORTED_VERSION: i64 = -32022;

/// An error to send.
#[derive(Clone, Debug, PartialEq)]
pub struct RpcError {
    /// The code.
    pub code: i64,
    /// The message.
    pub message: String,
    /// More, if any.
    pub data: Option<Value>,
}

impl RpcError {
    /// An error without data.
    #[must_use]
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// The same with data.
    #[must_use]
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// A request read from one line.
#[derive(Debug, PartialEq)]
pub struct Request {
    /// The id; `None` for a notification.
    pub id: Option<Value>,
    /// The method.
    pub method: String,
    /// The params (an empty object if there are none).
    pub params: Value,
}

/// Read a line. An error carries the answer line to send (an invalid request is answered, with a null id if it has none).
///
/// # Errors
/// The line is not a JSON-RPC 2.0 request; the text is the complete error response.
pub fn parse(line: &str) -> Result<Request, String> {
    let v: Value = serde_json::from_str(line).map_err(|e| {
        error(
            &Value::Null,
            &RpcError::new(PARSE_ERROR, format!("not JSON: {e}")),
        )
    })?;
    let invalid = |why: &str| error(&Value::Null, &RpcError::new(INVALID_REQUEST, why));
    let Value::Object(m) = v else {
        return Err(invalid(
            "a request is one JSON object (batches are not supported)",
        ));
    };
    if m.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(invalid("`jsonrpc` must be \"2.0\""));
    }
    let id = match m.get("id") {
        None => None,
        Some(i @ (Value::String(_) | Value::Number(_))) => Some(i.clone()),
        Some(_) => return Err(invalid("`id` must be a string or a number")),
    };
    let Some(method) = m.get("method").and_then(Value::as_str) else {
        return Err(error(
            id.as_ref().unwrap_or(&Value::Null),
            &RpcError::new(INVALID_REQUEST, "`method` must be a string"),
        ));
    };
    let params = match m.get("params") {
        None => Value::Object(Map::new()),
        Some(p @ Value::Object(_)) => p.clone(),
        Some(_) => {
            return Err(error(
                id.as_ref().unwrap_or(&Value::Null),
                &RpcError::new(INVALID_PARAMS, "`params` must be an object"),
            ));
        }
    };
    Ok(Request {
        id,
        method: method.to_owned(),
        params,
    })
}

/// The line of a result.
#[must_use]
pub fn result(id: &Value, result: Value) -> String {
    json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string()
}

/// The line of an error.
#[must_use]
pub fn error(id: &Value, e: &RpcError) -> String {
    let mut err = json!({"code": e.code, "message": e.message});
    if let Some(d) = &e.data {
        err["data"] = d.clone();
    }
    json!({"jsonrpc": "2.0", "id": id, "error": err}).to_string()
}
