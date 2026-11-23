//! Test providers, conformance rules and the fixed sample data of FreeSynergy groups (pilot `programm-grundgeruest`,
//! step K0). The frontends develop against [`FakeGroups`]; the core agent owns this crate.
//!
//! Pattern: Fake behind a Repository trait (the contract is the folder [`groups`]: ONE contract, no versions side by side).
//!
//! * [`groups`]: the contract (members, edges with a mode per edge, rights, owners and three levels of ADR 0095,
//!   addons by composition, the ports `PersonFacts` and `RoleBook`); the handshake number is 1.
//! * [`FakeGroups`]: in memory, every method and rule of `Groups`, `GroupGovernance`, `Mirroring` and `GroupMembers`;
//!   [`EmptyGroups`] the empty one; [`FakePeople`] the consumed fs-people side; [`FakeMirror`] the Kanidm side.
//! * [`sample`]: the fixed set (15 groups and 20 persons, a `TopDown` chain of nine levels, every case of the plan);
//!   [`example_data`]: invented example data in the structure of a self-managed network (fixture file, provider fake).
//! * [`conformance`]: rules any provider must pass (`run_conformance`, `run_conformance_governance`).
//! * `nesting_logic` (private): the pure nesting rule, the seed of the later crate `group-nesting`.

// Allowed for the whole crate, with the reason: the state methods are long argument lists of the trait itself, and
// the fake's modules repeat the names of the trait items on purpose (`ops_member` holds the member operations).
#![allow(clippy::module_name_repetitions, clippy::too_many_arguments)]

pub mod groups;

mod access;
mod bridge;
pub mod conformance;
mod decorate;
mod decorate_groups;
mod decorate_other;
mod doubles;
mod example;
mod example_data;
mod fake;
mod fake_governance;
mod fake_impl;
mod fake_mirror;
mod fake_operator;
mod govern_state;
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
mod ops_operator;
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
mod validate;

pub use decorate::{Decorated, Gate, Op, Provider};
pub use doubles::{Clock, FailGate, FailingGroups, SlowGate, SlowGroups, forbidden, unavailable};
pub use example::{EXAMPLE_LOCAL_MEMBERS, Example, example};
pub use example_data::{EXAMPLE_MARK, ROLE_TEMPLATES, RoleTemplate};
pub use fake::{DECISIONS, EmptyGroups, FakeGroups, START_MS};
pub use fake_mirror::FakeMirror;
pub use ids::{group_ref, id_of_reference, object_id, object_number, person, reference_of};
pub use people::FakePeople;
pub use sample::{Sample, sample};
