//! Conformance rules of `groups`, runnable against any provider (the fake, `LocalGroups`, `RemoteGroups`).
//!
//! Pattern: Template method (a fixed list of rules, each one checked against the `Groups` under test).
//!
//! `run_conformance` takes a [`ConformanceFixture`]: the provider plus the few things the trait cannot do (make a
//! human, an AI or an unknown person, say who asks, move the clock). It seeds its own data through the trait (every
//! group is named `Zzconf ...`), so it works on an empty or a filled provider. A mutator a fixture cannot offer
//! keeps its default and the rules that need it are reported as skipped, not as passed.

// The rules are flat lists of checks on a few groups and persons; splitting a list hides nothing, and the names
// (`a`, `b`, `p`, `q`) are the groups and persons of ONE rule. Hence these two pedantic lints are off here.
#![allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod cx;
mod fixture;
mod rules_addon;
mod rules_addon_more;
mod rules_deep;
mod rules_directions;
mod rules_flow_owner;
mod rules_groups;
mod rules_hidden;
mod rules_hidden_ids;
mod rules_hidden_sub;
mod rules_invites;
mod rules_leaving;
mod rules_list;
mod rules_members;
mod rules_mirror;
mod rules_mirror_more;
mod rules_mirror_trees;
mod rules_nesting;
mod rules_nesting_more;
mod rules_operator;
mod rules_operator_local;
mod rules_operator_names;
mod rules_owner;
mod rules_owner_exit;
mod rules_owner_more;
mod rules_owner_roles;
mod rules_requests;
mod rules_requests_more;
mod rules_rights;
mod rules_scope;
mod rules_scope_ai;
mod rules_trash;
mod run;

pub use cx::Report;
pub use fixture::{ByHand, ConformanceFixture, Kind};
pub use run::{run_conformance, run_conformance_governance};
