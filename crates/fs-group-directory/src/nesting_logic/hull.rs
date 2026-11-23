//! The hull of effective members: which groups feed a group over the arcs of the member graph, and by which way.
//!
//! Pattern: Specification (one breadth-first search backwards from the asked group; the first find per source is
//! the shortest way).

use super::graph::{Edge, arcs};
use std::collections::{BTreeMap, VecDeque};

/// A group whose direct members count in the asked group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Source<I> {
    pub group: I,
    /// The shortest way `group` to the asked group (both ends included).
    pub via: Vec<I>,
}

/// The asked group first (way of one), then every group with an arc way into it, nearest first.
pub(crate) fn hull<I: Copy + Ord>(edges: &[Edge<I>], group: I) -> Vec<Source<I>> {
    let pairs = arcs(edges);
    let mut via: BTreeMap<I, Vec<I>> = BTreeMap::from([(group, vec![group])]);
    let mut order = vec![group];
    let mut queue = VecDeque::from([group]);
    while let Some(at) = queue.pop_front() {
        for (u, v) in &pairs {
            if *v == at && !via.contains_key(u) {
                let mut way = vec![*u];
                way.extend(via[&at].iter().copied());
                via.insert(*u, way);
                order.push(*u);
                queue.push_back(*u);
            }
        }
    }
    order
        .into_iter()
        .map(|g| Source {
            group: g,
            via: via[&g].clone(),
        })
        .collect()
}
