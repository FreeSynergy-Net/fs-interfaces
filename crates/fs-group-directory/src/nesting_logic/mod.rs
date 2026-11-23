//! The pure nesting logic of the fake: edges with a mode, the member graph, cycles, depth, the hull of effective
//! members. It is the seed of the later crate `group-nesting` and is replaced by it; it knows nothing of groups as
//! rows, of people or of rights, only of ids and edges.
//!
//! Pattern: Specification (the rule of `nesting.rs` as pure functions over an edge list).

mod graph;
mod hull;

pub(crate) use graph::{Edge, Mode, arcs, find_cycle, reach, too_deep};
pub(crate) use hull::hull;
