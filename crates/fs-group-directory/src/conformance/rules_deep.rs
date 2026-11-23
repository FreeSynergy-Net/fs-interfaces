//! Rules: the depth limit counts in the MEMBER graph too, not only in the tree (zigzag chain of changing directions).
//!
//! Pattern: Template method (a fixed list of rules; the groups are seeded through the trait).

use super::cx::{Cx, err};
use crate::groups_v1::{Error, MAX_NESTING_DEPTH, NestingMode};

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    zigzag(cx).await;
    topdown_chain(cx).await;
}

/// `g0 -> g1 -> ... -> gN` in the member graph, but the tree is flat: the edges alternate `TopDown` (the lower group
/// is the child) and `BottomUp` (the lower group is the parent), so no tree chain is longer than one edge, while the
/// chain of members is as long as the list. 16 arcs are allowed, the 17th is `TooDeep`.
async fn zigzag(cx: &mut Cx<'_>) {
    let last = MAX_NESTING_DEPTH as usize + 1;
    let mut chain = Vec::new();
    for i in 0..=last {
        let Some(g) = cx.group(&format!("zz{i}")).await else {
            return;
        };
        chain.push(g);
    }
    for (i, w) in chain
        .windows(2)
        .take(MAX_NESTING_DEPTH as usize)
        .enumerate()
    {
        // The arc `w[0] -> w[1]`: TopDown when the upper group is the parent, BottomUp when it is the child.
        let linked = if i % 2 == 0 {
            cx.link(w[1].id, w[0].id, NestingMode::TopDown).await
        } else {
            cx.link(w[0].id, w[1].id, NestingMode::BottomUp).await
        };
        if linked.is_none() {
            return;
        }
    }
    let n = chain.len();
    let r =
        cx.g.request_nesting(chain[n - 1].id, chain[n - 2].id, NestingMode::TopDown)
            .await;
    cx.ensure(
        matches!(err(&r), Some(Error::TooDeep { max }) if *max == MAX_NESTING_DEPTH),
        "depth: a zigzag chain (flat tree, 17 arcs in the member graph) is TooDeep",
    );
    let r =
        cx.g.nesting_effect(chain[n - 1].id, chain[n - 2].id, NestingMode::TopDown)
            .await;
    cx.ensure(
        matches!(err(&r), Some(Error::TooDeep { max }) if *max == MAX_NESTING_DEPTH),
        "depth: nesting_effect says TooDeep for the zigzag chain too",
    );
}

/// The same limit for a plain `TopDown` chain (the arcs run against the edges).
async fn topdown_chain(cx: &mut Cx<'_>) {
    let mut chain = Vec::new();
    for i in 0..=MAX_NESTING_DEPTH + 1 {
        let Some(g) = cx.group(&format!("tdd{i}")).await else {
            return;
        };
        chain.push(g);
    }
    for w in chain.windows(2).take(MAX_NESTING_DEPTH as usize) {
        cx.link(w[1].id, w[0].id, NestingMode::TopDown).await;
    }
    let n = chain.len();
    let r =
        cx.g.request_nesting(chain[n - 1].id, chain[n - 2].id, NestingMode::TopDown)
            .await;
    cx.ensure(
        matches!(err(&r), Some(Error::TooDeep { max }) if *max == MAX_NESTING_DEPTH),
        "depth: a TopDown chain of more than 16 edges is TooDeep",
    );
}
