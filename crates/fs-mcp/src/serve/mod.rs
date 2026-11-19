//! MCP stage 2: the server of the revision 2026-07-28 over stdio. It is the only place that speaks the protocol; the tools
//! come from the generated table (`capnpc-mcp`), the way to the program from the generated command code (`capnpc-cli`).
//!
//! Pattern: Facade — [`Server`] hides the stages: lines ([`rpc`]), what every request carries ([`lifecycle`]), the tools
//! ([`tools`]), arguments and results ([`input`], [`output`], [`fit`]). Strategy: [`calls::Calls`] (which program),
//! [`switch::Switch`] (on or off). Off is the default: with an off switch the server does not start, writes nothing to
//! stdout, and says why on stderr. The list of tools is [`crate::Surface::admit`], never the table itself.

pub mod base64;
pub mod calls;
pub mod fit;
pub mod input;
pub mod lifecycle;
pub mod output;
pub mod rpc;
pub mod schema;
pub mod switch;
pub mod tools;

use std::io::{BufRead, Write};

use capnp_compiler::mcpgen::rt::ToolSpec;
use futures::executor::LocalPool;
use serde_json::Value;

use crate::{Catalog, Surface};
pub use calls::{Calls, TasksCalls};
pub use lifecycle::ServerInfo;
use rpc::{METHOD_NOT_FOUND, RpcError};
pub use switch::{Never, Off, Started, Switch};

/// Why a server did not start.
#[derive(Debug, PartialEq, Eq)]
pub enum StartError {
    /// The switch is off (the default, and always for the vault).
    SwitchOff,
    /// A schema of the table is not JSON.
    BadTable(String),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SwitchOff => f.write_str("the MCP server is off: a human turns it on on purpose"),
            Self::BadTable(s) => write!(f, "the tool table is broken: {s}"),
        }
    }
}

/// A server for one table of tools and one program.
pub struct Server<C> {
    calls: C,
    offered: Vec<tools::Offered>,
    info: ServerInfo,
}

impl<C: Calls> Server<C> {
    /// A server that offers the tools of `table` that the switch and the screen let through.
    ///
    /// # Errors
    /// [`StartError::SwitchOff`] if the switch is off; nothing is offered then and nothing is to be written anywhere.
    pub fn new(
        calls: C,
        table: &'static [ToolSpec],
        catalog: &Catalog,
        switch: &dyn Switch,
        info: ServerInfo,
    ) -> Result<Self, StartError> {
        if !switch.is_on() {
            return Err(StartError::SwitchOff);
        }
        let admitted = Surface::new(catalog).admit(switch, table);
        let offered = admitted
            .offered
            .into_iter()
            .map(tools::Offered::new)
            .collect::<Result<Vec<_>, _>>()
            .map_err(StartError::BadTable)?;
        Ok(Self {
            calls,
            offered,
            info,
        })
    }

    /// The names of the tools offered, in the order of the list.
    #[must_use]
    pub fn tool_names(&self) -> Vec<&'static str> {
        self.offered.iter().map(|t| t.spec.name).collect()
    }

    /// Answer one line; `None` for a notification, an empty line, or anything that is not to be answered.
    pub async fn handle_line(&self, line: &str) -> Option<String> {
        if line.trim().is_empty() {
            return None;
        }
        let req = match rpc::parse(line) {
            Ok(r) => r,
            Err(answer) => return Some(answer),
        };
        // a notification has no id and is never answered
        let id = req.id?;
        Some(match self.dispatch(&req.method, &req.params).await {
            Ok(v) => rpc::result(&id, v),
            Err(e) => rpc::error(&id, &e),
        })
    }

    async fn dispatch(&self, method: &str, params: &Value) -> Result<Value, RpcError> {
        if method == "initialize" {
            return Err(lifecycle::legacy_initialize());
        }
        let meta = lifecycle::check_meta(params)?;
        match method {
            "server/discover" => Ok(lifecycle::discover(&self.info, &meta)),
            "ping" => Ok(lifecycle::ping(&self.info, &meta)),
            "tools/list" => Ok(tools::list(&self.offered, &self.info, &meta)),
            "tools/call" => {
                tools::call(&self.calls, &self.offered, params, &self.info, &meta).await
            }
            other => Err(RpcError::new(
                METHOD_NOT_FOUND,
                format!("no such method: {other}"),
            )),
        }
    }

    /// Read lines until the input ends, answer each on `out` (one line, flushed). The program spawns whatever its
    /// connection needs on `pool` before; the calls of a line are run to the end on it, one line after the other.
    ///
    /// # Errors
    /// Reading or writing fails.
    pub fn serve<R: BufRead, W: Write>(
        &self,
        pool: &mut LocalPool,
        input: R,
        mut out: W,
    ) -> std::io::Result<()> {
        for line in input.lines() {
            if let Some(answer) = pool.run_until(self.handle_line(&line?)) {
                writeln!(out, "{answer}")?;
                out.flush()?;
            }
        }
        Ok(())
    }
}
