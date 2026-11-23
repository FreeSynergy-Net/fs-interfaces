//! `FakeStorage`: an in-memory provider with the sample tree, for frontends and the conformance rules.
//!
//! Pattern: Fake (a real, small implementation of the trait; faults are switchable for the mutant tests).

mod access;
mod local;
mod naive;
mod provider_impl;
mod read;
mod sample;
mod state;
mod trash;
mod write;

pub use local::FakeLocal;
pub use sample::{GROUP_NAME, Sample, sample};
pub use state::{FakeStorage, Fault};
