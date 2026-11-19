//! A stand-in for the vault, until there is one: its own table of tools, the contract the catalog is made from, and the
//! switch rule. Test helpers; no pattern of their own.
//!
//! In production the vault's switch is the type `Never`: no parameter, no feature, no setting. In the test MCP is on for the
//! vault too (`Started`), so the screen alone has to keep every secret out.
#![allow(dead_code)]

use capnp_compiler::mcpgen::rt::{Danger, Never, ToolSpec};
use fs_mcp::serve::{Server, ServerInfo, StartError, Started};
use fs_mcp::{
    Blocklist, Catalog, Contract, Field, Generator, Interface, Mark, Method, RiskClass, RiskSource,
    Target, ToolGenerator, TypeRef,
};

use super::calls::StoreCalls;

const FILE: &str = "vault";

fn field(name: &str, doc: &str, ty: TypeRef) -> Field {
    Field {
        name: name.into(),
        doc: doc.into(),
        ty,
        in_union: false,
    }
}

fn method(name: &str, params: Vec<Field>, results: Vec<Field>) -> Method {
    Method {
        name: name.into(),
        doc: format!("{name} of the vault"),
        params,
        results,
        streaming: false,
        unclear: false,
    }
}

/// The contract of the stand-in: tools whose names or comments say "secret" and tools that do not.
pub fn contract() -> Vec<Contract> {
    let text = || TypeRef::Text;
    vec![Contract {
        file: FILE.into(),
        interfaces: vec![Interface {
            name: "Vault".into(),
            doc: String::new(),
            methods: vec![
                method(
                    "get",
                    vec![field("name", "", text())],
                    vec![
                        field("name", "", text()),
                        field("value", "the secret value of the entry", text()),
                    ],
                ),
                method(
                    "put",
                    vec![
                        field("name", "", text()),
                        field("secret", "the secret to store", text()),
                    ],
                    vec![field("ok", "", TypeRef::Bool)],
                ),
                method(
                    "list",
                    vec![],
                    vec![field("names", "", TypeRef::List(Box::new(text())))],
                ),
                method(
                    "export",
                    vec![],
                    vec![field("blob", "the whole vault", TypeRef::Data)],
                ),
                method(
                    "rotate",
                    vec![],
                    vec![
                        field("key", "the new encryption key", TypeRef::Data),
                        field(
                            "version",
                            "",
                            TypeRef::Int {
                                unsigned: true,
                                bits: 32,
                            },
                        ),
                    ],
                ),
                method(
                    "unlock",
                    vec![field("passphrase", "", text())],
                    vec![field("ok", "", TypeRef::Bool)],
                ),
                method(
                    "token",
                    vec![],
                    vec![field("token", "an access token", text())],
                ),
                method(
                    "describe",
                    vec![field("name", "", text())],
                    vec![
                        field("kind", "", text()),
                        field(
                            "created",
                            "",
                            TypeRef::Int {
                                unsigned: false,
                                bits: 64,
                            },
                        ),
                    ],
                ),
            ],
        }],
        structs: vec![],
        enums: vec![],
    }]
}

/// Marks: every method is `read`. With `release_all` a human has wrongly released every place (the self-test of the
/// driver: it must then find the leaks).
pub struct VaultRisk {
    pub release_all: bool,
}

impl RiskSource for VaultRisk {
    fn mark(&self, t: &Target) -> Option<Mark> {
        t.as_str().starts_with("vault/").then(|| Mark {
            class: RiskClass::Read,
            capability: None,
            resource: false,
            reason: None,
        })
    }
    fn release(&self, _: &Target) -> Option<String> {
        self.release_all.then(|| "released by mistake".to_owned())
    }
    fn format(&self, _: &Target) -> Option<String> {
        self.release_all.then(|| "released by mistake".to_owned())
    }
    fn targets(&self) -> Vec<Target> {
        Vec::new()
    }
}

fn spec(name: &str, method: &str, input: &str, output: &str) -> ToolSpec {
    let leak = |s: String| -> &'static str { Box::leak(s.into_boxed_str()) };
    ToolSpec {
        name: leak(name.to_owned()),
        title: leak(name.to_owned()),
        description: leak(format!("{method} of the vault")),
        key: leak(format!("vault/Vault.{method}")),
        member: None,
        words: Box::leak(vec![leak(method.to_owned())].into_boxed_slice()),
        input_schema: leak(input.to_owned()),
        output_schema: leak(output.to_owned()),
        meta: r#"{"net.freesynergy/mcp":{"actor":"ai","risk":"read"}}"#,
        risk: Some(Danger::Read),
        free_content: false,
    }
}

const NONE_IN: &str = r#"{"type":"object","additionalProperties":false}"#;
const NAME_IN: &str = r#"{"type":"object","properties":{"name":{"type":"string"}},"required":["name"],"additionalProperties":false}"#;

/// The table of the stand-in: every tool that the generator would write for the contract above.
pub fn table() -> &'static [ToolSpec] {
    let obj = |props: &str| format!(r#"{{"type":"object","properties":{{{props}}}}}"#);
    let t = vec![
        spec(
            "vault.get",
            "get",
            NAME_IN,
            &obj(
                r#""name":{"type":"string"},"value":{"type":"string","description":"the secret value of the entry"}"#,
            ),
        ),
        spec(
            "vault.put",
            "put",
            r#"{"type":"object","properties":{"name":{"type":"string"},"secret":{"type":"string","description":"the secret to store"}},"required":["name","secret"],"additionalProperties":false}"#,
            &obj(r#""ok":{"type":"boolean"}"#),
        ),
        spec(
            "vault.list",
            "list",
            NONE_IN,
            &obj(r#""names":{"type":"array","items":{"type":"string"}}"#),
        ),
        spec(
            "vault.export",
            "export",
            NONE_IN,
            &obj(
                r#""blob":{"type":"string","contentEncoding":"base64","description":"the whole vault"}"#,
            ),
        ),
        spec(
            "vault.rotate",
            "rotate",
            NONE_IN,
            &obj(
                r#""key":{"type":"string","contentEncoding":"base64","description":"the new encryption key"},"version":{"type":"integer","minimum":0}"#,
            ),
        ),
        spec(
            "vault.unlock",
            "unlock",
            r#"{"type":"object","properties":{"passphrase":{"type":"string"}},"required":["passphrase"],"additionalProperties":false}"#,
            &obj(r#""ok":{"type":"boolean"}"#),
        ),
        spec(
            "vault.token",
            "token",
            NONE_IN,
            &obj(r#""token":{"type":"string","description":"an access token"}"#),
        ),
        spec(
            "vault.describe",
            "describe",
            NAME_IN,
            &obj(
                r#""kind":{"type":"string"},"created":{"type":"string","pattern":"^-?[0-9]+$","x-fs-int":{"signed":true,"bits":64,"minimum":"-9223372036854775808","maximum":"9223372036854775807"}}"#,
            ),
        ),
    ];
    Box::leak(t.into_boxed_slice())
}

/// The catalog of the stand-in, as the screen makes it.
pub fn catalog(release_all: bool) -> Catalog {
    Generator::new(&VaultRisk { release_all }, Blocklist::standard())
        .generate(&contract())
        .expect("generation works")
}

pub fn info() -> ServerInfo {
    ServerInfo {
        name: "vault-stand-in".into(),
        version: "0".into(),
    }
}

/// The PRODUCTION path of the vault: it takes no switch. Its switch is `Never`, a type without state.
pub fn production_server(
    calls: StoreCalls,
    catalog: &Catalog,
) -> Result<Server<StoreCalls>, StartError> {
    Server::new(calls, table(), catalog, &Never, info())
}

/// The path of the TEST: MCP is on for the vault too, so that the screen alone is what keeps the secrets out.
pub fn test_server(calls: StoreCalls, catalog: &Catalog) -> Result<Server<StoreCalls>, StartError> {
    Server::new(calls, table(), catalog, &Started, info())
}
