//! The contract `module_v1` of FreeSynergy: what a module service (planning, settings, ...) must do to be drawn and written by a
//! host. It is `FormSource` level 1 (`hello`, `page`, `blob`, `invoke`) with the rules written in `schema/formsource.capnp`.
//!
//! This crate has three parts: [`FormSourceFixture`] (what a service under test tells the suite), [`FakeSource`] (a small, complete
//! source with example data) and [`conformance`] (the 21 rules, run against any fixture; level 2 adds the rules L2-2 to L2-11). It does not depend on `fs-rpc`: an RPC
//! exception is checked by its text (`form-source-<kind>`); the full decoding of the problem is tested by the host.

pub mod conformance;
mod fake;
mod fake_l2;
mod fake_server;
mod fixture;

pub use fake::{FakeSource, IMAGE, Mode, Refusal};
pub use fixture::{
    BlobSample, CommandKind, DeclaredCommand, DeclaredForm, FormSourceFixture, LaneCase, Ordered,
    Param, Sample, Verify,
};
