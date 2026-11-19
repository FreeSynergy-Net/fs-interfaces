//! What every request carries and what the server says about itself: `_meta` per request, `server/discover`, `ping`.
//!
//! Pattern: Specification — the revision 2026-07-28 is stateless: there is no `initialize` and no session, every request
//! says the protocol version and the client capabilities in its `_meta`. Who acts and with which rights comes from the
//! connection (the key the human put into the environment of the process), never from `_meta`; `clientInfo.name` is a label
//! for the actor (`ai:<name>`) and decides nothing.

use capnp_compiler::mcpgen::rt::{self, PROTOCOL_VERSION};
use serde_json::{Value, json};

use super::rpc::{INVALID_PARAMS, METHOD_NOT_FOUND, RpcError, UNSUPPORTED_VERSION};

const VERSION_KEY: &str = "io.modelcontextprotocol/protocolVersion";
const CAPABILITIES_KEY: &str = "io.modelcontextprotocol/clientCapabilities";
const SERVER_INFO_KEY: &str = "io.modelcontextprotocol/serverInfo";

/// What a request said about its client.
#[derive(Debug, PartialEq, Eq)]
pub struct Meta {
    /// `clientInfo.name`, self-reported.
    pub client_name: Option<String>,
}

/// The name and version of this server, self-reported to the client.
#[derive(Clone, Debug)]
pub struct ServerInfo {
    /// Name.
    pub name: String,
    /// Version.
    pub version: String,
}

/// The error for the legacy handshake: there is none in this revision, and no session comes of it.
#[must_use]
pub fn legacy_initialize() -> RpcError {
    RpcError::new(
        METHOD_NOT_FOUND,
        format!(
            "there is no `initialize` in MCP {PROTOCOL_VERSION}: send `server/discover`, and put the protocol version and the client capabilities into the `_meta` of every request"
        ),
    )
}

/// Check the `_meta` of a request.
///
/// # Errors
/// `-32602` if the protocol version or the client capabilities are missing (the data names them), `-32022` with
/// `data.supported` for another version.
pub fn check_meta(params: &Value) -> Result<Meta, RpcError> {
    let meta = params.get("_meta").and_then(Value::as_object);
    let version = meta
        .and_then(|m| m.get(VERSION_KEY))
        .and_then(Value::as_str);
    let caps = meta
        .and_then(|m| m.get(CAPABILITIES_KEY))
        .filter(|c| c.is_object());
    let mut missing = Vec::new();
    if version.is_none() {
        missing.push(VERSION_KEY);
    }
    if caps.is_none() {
        missing.push(CAPABILITIES_KEY);
    }
    if !missing.is_empty() {
        return Err(RpcError::new(
            INVALID_PARAMS,
            "the `_meta` of the request lacks a required field",
        )
        .with_data(json!({ "missing": missing })));
    }
    let version = version.unwrap_or_default();
    if version != PROTOCOL_VERSION {
        return Err(
            RpcError::new(UNSUPPORTED_VERSION, "Unsupported protocol version")
                .with_data(json!({ "supported": [PROTOCOL_VERSION], "requested": version })),
        );
    }
    let client_name = meta
        .and_then(|m| m.get("io.modelcontextprotocol/clientInfo"))
        .and_then(|i| i.get("name"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    Ok(Meta { client_name })
}

/// The `_meta` of a result: who the server is, and the actor label of the AI (always `ai:`).
#[must_use]
pub fn result_meta(info: &ServerInfo, meta: &Meta) -> Value {
    let actor = rt::actor(meta.client_name.as_deref());
    let ours: Value = serde_json::from_str(&rt::result_meta(&actor)).unwrap_or(Value::Null);
    let mut m = ours.as_object().cloned().unwrap_or_default();
    m.insert(
        SERVER_INFO_KEY.to_owned(),
        json!({"name": info.name, "version": info.version}),
    );
    Value::Object(m)
}

/// The result of `server/discover`.
#[must_use]
pub fn discover(info: &ServerInfo, meta: &Meta) -> Value {
    json!({
        "resultType": rt::RESULT_TYPE,
        "supportedVersions": [PROTOCOL_VERSION],
        "capabilities": {"tools": {}},
        "_meta": result_meta(info, meta),
    })
}

/// The result of `ping`.
#[must_use]
pub fn ping(info: &ServerInfo, meta: &Meta) -> Value {
    json!({"resultType": rt::RESULT_TYPE, "_meta": result_meta(info, meta)})
}
