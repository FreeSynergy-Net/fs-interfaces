//! The 13 rules of `module_v1` (`FormSource` level 1), runnable against any [`FormSourceFixture`](crate::FormSourceFixture).
//!
//! Pattern: Template Method (a fixed list of rules; the fixture supplies client, declared forms and valid calls). A rule the fixture
//! cannot set up is reported as skipped, never as passed. Rule 13 (the counter-proof: a fake that breaks a rule must turn red) is
//! the self test in `tests/selftest.rs`.
//!
//! Deviation (honest): no `fs-rpc` here, so an RPC exception is checked by "exception text contains `form-source-<kind>`".

#![allow(clippy::many_single_char_names)]

mod cx;
mod rules_errors;
mod rules_read;
mod rules_rights;

mod run;

pub use cx::Report;
pub use run::run_conformance;
