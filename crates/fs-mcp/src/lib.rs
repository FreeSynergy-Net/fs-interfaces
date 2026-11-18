//! MCP tool generator, stage 1 (ADR 0021 extension, ADR 0054): lists the tools and resource templates
//! that the Cap'n Proto contracts allow, with a risk class each. It runs nothing: no server, no bridge,
//! no call. Eine Liste ist keine Erlaubnis: the list is an offer, the engine decides on every call.
//! Pattern: Strategy (see `DESIGN.md`).
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

mod blocklist;
mod capnpc_source;
mod catalog;
mod error;
mod free_format;
mod generator;
mod jsonschema;
mod mark_parse;
mod marks;
mod model;
mod program_marks;
mod resource;
mod risk;
mod secrets;
mod source;
mod surface;
mod target;
mod tool;

/// The command code of `tasks.capnp` (`capnpc-cli`) and its tool table (`capnpc-mcp`), written by `cargo run -p regenerate`
/// with the marks of this crate. The tool table names `crate::tasks_cli`, so both sit at the root of the crate.
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
#[path = "generated/tasks_cli.rs"]
pub mod tasks_cli;
#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
#[path = "generated/tasks_mcp.rs"]
pub mod tasks_mcp;

pub use blocklist::Blocklist;
pub use capnpc_source::CapnpcSource;
pub use catalog::{Catalog, Left, Why};
pub use error::Error;
pub use free_format::looks_free_format;
pub use generator::{Generator, ToolGenerator};
pub use jsonschema::SchemaBuilder;
pub use marks::MarkFiles;
pub use model::{Contract, EnumDef, Field, Index, Interface, Method, StructDef, TypeName, TypeRef};
pub use program_marks::ProgramMarks;
pub use resource::{ResourceKind, ResourceTemplate};
pub use risk::{Mark, RiskClass, RiskSource};
pub use secrets::{doc_looks_secret, name_looks_secret};
pub use source::ContractSource;
pub use surface::{Admission, Refusal, Surface};
pub use target::Target;
pub use tool::{META_KEY, Tool};
