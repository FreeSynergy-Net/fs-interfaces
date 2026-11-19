//! The way from a tool of the table to the command code that calls the program.
//!
//! Pattern: Strategy — [`Calls`] hides which generated file and which client a tool runs against; the server only
//! hands over the tool and the input/output. [`TasksCalls`] is the implementation for `tasks.capnp`.

use std::future::Future;
use std::pin::Pin;

use capnp_compiler::cligen::rt::{CallError, Input, Outcome, Output};
use capnp_compiler::mcpgen::rt::ToolSpec;
use fs_interfaces::tasks_capnp::tasks;

use crate::tasks_mcp;

/// A call in progress.
pub type Call<'a> = Pin<Box<dyn Future<Output = Result<Outcome, CallError>> + 'a>>;

/// Runs the command of a tool.
pub trait Calls {
    /// Start the command of `tool`; `None` if there is none (the table and the command code do not belong together).
    fn call<'a>(
        &'a self,
        tool: &ToolSpec,
        input: &'a mut dyn Input,
        output: &'a mut dyn Output,
    ) -> Option<Call<'a>>;
}

/// The commands of `tasks.capnp` against a client of the root interface (a connection of the program).
pub struct TasksCalls(pub tasks::Client);

impl Calls for TasksCalls {
    fn call<'a>(
        &'a self,
        tool: &ToolSpec,
        input: &'a mut dyn Input,
        output: &'a mut dyn Output,
    ) -> Option<Call<'a>> {
        let run = tasks_mcp::runner(tool)?;
        Some(run(&self.0, input, output))
    }
}
