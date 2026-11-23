//! Test providers, conformance rules and the fixed sample data of FreeSynergy groups (pilot `programm-grundgeruest`,
//! step K0). The frontends develop against [`FakeGroups`]; the core agent owns this crate.
//!
//! Pattern: Fake behind a locked Repository trait (the trait is the folder [`groups_v1`], byte for byte the locked one).
//!
//! * [`groups_v1`]: the LOCKED trait folder (SHA-256 91c0f8b0...2faf3); never edit it, a change is `groups_v2`.
//! * [`groups_v2`]: the contract v2 (owners and three levels of ADR 0095, `default_mode` for edges, addons by composition,
//!   the ports `PersonFacts` and `RoleBook`), next to v1; `FakeGroups` implements `GroupsV2` too, `run_conformance_v2` checks it.
//! * [`FakeGroups`]: in memory, every method and rule of `Groups`, `Mirroring` and `GroupMembers`; [`EmptyGroups`] the
//!   empty one; [`FakePeople`] the consumed fs-people side; [`FakeMirror`] the Kanidm side.
//! * [`sample`]: the fixed set (15 groups and 20 persons, a `TopDown` chain of nine levels, every case of the plan).
//! * [`conformance`]: rules any provider must pass (`run_conformance`, later also for `LocalGroups`).
//! * `nesting_logic` (private): the pure nesting rule, the seed of the later crate `group-nesting`.

// Allowed for the whole crate, with the reason: the state methods are long argument lists of the trait itself, and
// the fake's modules repeat the names of the trait items on purpose (`ops_member` holds the member operations).
#![allow(clippy::module_name_repetitions, clippy::too_many_arguments)]

// The locked folder: byte for byte the lock (`.scratch/sperren/groups_v1.sha256`). It was written for the Rust 2021
// probe and compiled with pedantic lints there; the allows below are the ones the 2024 workspace needs, outside it.
// `rustfmt::skip`: the folder stays byte for byte as locked (it was formatted for the 2021 probe).
#[rustfmt::skip]
#[allow(missing_docs, clippy::doc_markdown, clippy::must_use_candidate, clippy::large_enum_variant, clippy::wildcard_imports)]
pub mod groups_v1;

/// The contract v2 (owners, levels, addons), next to the locked v1; see its module doc.
#[allow(clippy::module_name_repetitions)]
pub mod groups_v2;

mod access;
mod bridge;
pub mod conformance;
mod decorate;
mod decorate_groups;
mod decorate_other;
mod doubles;
mod fake;
mod fake_impl;
mod fake_mirror;
mod fake_v2;
mod ids;
mod mirror_apply;
mod mirror_plan;
mod mirror_run;
mod mirror_state;
mod nest_check;
mod nesting_logic;
mod ops_addon;
mod ops_facade;
mod ops_feed;
mod ops_group;
mod ops_invite;
mod ops_member;
mod ops_nest;
mod ops_owner;
mod ops_request;
mod ops_right;
mod ops_scope;
mod ops_trash;
mod ops_tree;
mod ops_write;
mod paging;
mod people;
mod sample;
mod sample_activity;
mod sample_data;
mod state;
mod v2_state;
mod validate;

pub use decorate::{Decorated, Gate, Op, Provider};
pub use doubles::{Clock, FailGate, FailingGroups, SlowGate, SlowGroups, forbidden, unavailable};
pub use fake::{DECISIONS, EmptyGroups, FakeGroups, START_MS};
pub use fake_mirror::FakeMirror;
pub use ids::{group_ref, id_of_reference, object_id, object_number, person, reference_of};
pub use people::FakePeople;
pub use sample::{Sample, sample};
