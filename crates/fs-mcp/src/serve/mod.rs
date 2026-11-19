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
pub mod server;
pub mod switch;
pub mod tools;

pub use calls::{Calls, TasksCalls};
pub use lifecycle::ServerInfo;
pub use server::{Server, StartError};
pub use switch::{Never, Off, Started, Switch};
