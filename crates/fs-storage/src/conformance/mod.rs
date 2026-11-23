//! Conformance rules of `storage`: what every provider must do, run against the sample tree.
//!
//! Pattern: Template method (`run` runs every rule on a fresh fixture); rules are plain async functions.
//!
//! A provider passes the sample set (`fake::sample`): the rules find things by display path, so a real provider is
//! prepared with the same tree. Rights rules and name rules are the security part (ADR 0070 risks 1, 9).
//!
//! Two layers, one fake: the TABLE in `matrix` (ADR 0070, rights matrix; rows in `matrix/rows_*.rs`) is the source of the
//! exact result codes, one representative target per kind. The sweeps (`none_allowed`, `own_into_foreign`, `stranger_*`,
//! `member_readonly`, `rights.bot`) are "every instance" checks derived from it: they try EVERY entry of the tree and
//! expect only "Forbidden, tree unchanged". If one layer changes without the other, the suite turns red, never silently
//! green.

mod admin;
mod big;
mod changes;
mod fixture;
pub mod matrix;
mod names;
mod operator_log;
mod read;
mod rights;
mod run;
mod scope;
mod write;

pub use fixture::{Fixture, Report};
pub use run::run;
