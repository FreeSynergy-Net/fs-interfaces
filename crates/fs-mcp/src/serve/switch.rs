//! The switch of the server: off until a human turns it on.
//!
//! Pattern: Strategy — `capnp_compiler::mcpgen::rt::Switch`. The generated code is off (`Off`), the vault never turns on
//! (`Never`), and a program that starts the server because its human said so uses [`Started`]. Nothing here reads a
//! setting; the setting `fs.mcp.enabled` comes with the settings database (a later step).

pub use capnp_compiler::mcpgen::rt::{Never, Off, Switch};

/// On because the human started the server on purpose (the start flag of the process).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Started;

impl Switch for Started {
    fn is_on(&self) -> bool {
        true
    }
}
