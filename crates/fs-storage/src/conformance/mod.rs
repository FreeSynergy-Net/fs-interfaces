//! Conformance rules of `storage`: what every provider must do, run against the sample tree.
//!
//! Pattern: Template method (`run` runs every rule on a fresh fixture); rules are plain async functions.
//!
//! A provider passes the sample set (`fake::sample`): the rules find things by display path, so a real provider is
//! prepared with the same tree. Rights rules and name rules are the security part (ADR 0070 risks 1, 9).

mod admin;
mod changes;
mod fixture;
mod names;
mod read;
mod rights;
mod run;
mod write;

pub use fixture::{Fixture, Report};
pub use run::run;
