//! The checks of one edge (self edge, AI rule, cycle, depth, sensitivity) and `nesting_effect`.
//!
//! Pattern: Specification (one check function every writer of edges calls; the same rules at the request, at the
//! answer, at a mode change and at `restore`).

use crate::groups_v1::shared::ObjectId;
use crate::groups_v1::{
    ConflictKind, Error, InForce, MAX_NESTING_DEPTH, Nesting, NestingEffect, NestingMode, Result,
    Right,
};
use crate::ids::key;
use crate::nesting_logic::{Edge, find_cycle, hull, too_deep};
use crate::state::State;
use std::collections::{BTreeMap, BTreeSet};

impl State {
    /// All checks for the edge `child` to `parent` in `mode`, leaving out the edge `replacing` (a mode change).
    pub(crate) fn check_edge(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
        replacing: Option<ObjectId>,
    ) -> Result<()> {
        if child == parent {
            return Err(Error::Cycle {
                path: vec![child, child],
            });
        }
        let ai = |g: ObjectId| self.groups.get(&g).is_some_and(|r| r.group.ai);
        if ai(parent) || (ai(child) && mode != NestingMode::Single) {
            let group = if ai(parent) { parent } else { child };
            return Err(Error::AiNesting {
                group,
                path: vec![child, parent],
            });
        }
        let new = Self::raw_edge(child, parent, mode);
        let others: Vec<Edge<ObjectId>> = self
            .edges
            .iter()
            .filter(|e| Some(e.id) != replacing)
            .map(|e| self.logic_edge(e))
            .collect();
        if let Some(path) = find_cycle(&others, &new) {
            return Err(Error::Cycle { path });
        }
        if too_deep(&others, &new, MAX_NESTING_DEPTH) {
            return Err(Error::TooDeep {
                max: MAX_NESTING_DEPTH,
            });
        }
        if let Some((u, v)) = new.arc() {
            let sensitive = |g: ObjectId| self.groups.get(&g).is_some_and(|r| r.group.sensitive);
            if sensitive(u) != sensitive(v) {
                return Err(Error::ModeConflict(ConflictKind::SensitiveFlow {
                    from: u,
                    to: v,
                }));
            }
        }
        Ok(())
    }

    /// The edge of a pair, when there is one.
    pub(crate) fn edge_of(&self, child: ObjectId, parent: ObjectId) -> Option<&Nesting> {
        self.edges
            .iter()
            .find(|e| e.child == child && e.parent == parent)
    }

    /// Direct members that count, per living group, over the hull of `edges`.
    fn member_sets(&self, edges: &[Edge<ObjectId>]) -> BTreeMap<ObjectId, BTreeSet<String>> {
        let mut out = BTreeMap::new();
        for (g, row) in &self.groups {
            if row.deleted.is_some() {
                continue;
            }
            let mut set = BTreeSet::new();
            for src in hull(edges, *g) {
                for m in self.members.iter().filter(|m| m.group == src.group) {
                    if self.member_force(m) != InForce::KindMismatch {
                        set.insert(key(&m.person));
                    }
                }
            }
            out.insert(*g, set);
        }
        out
    }

    pub(crate) fn nesting_effect(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Result<NestingEffect> {
        self.visible(child)?;
        self.visible(parent)?;
        if !self.may_ask(child, Right::Manage) && !self.may_ask(parent, Right::Manage) {
            return Err(Error::Forbidden {
                group: child,
                right: Right::Manage,
            });
        }
        let existing = self.edge_of(child, parent);
        if let Some(e) = existing.filter(|e| e.mode == mode) {
            return Err(Error::AlreadyThere(e.id));
        }
        self.check_edge(child, parent, mode, existing.map(|e| e.id))?;
        let before_edges = self.logic_edges();
        let mut after_edges: Vec<Edge<ObjectId>> = self
            .edges
            .iter()
            .filter(|e| Some(e.id) != existing.map(|x| x.id))
            .map(|e| self.logic_edge(e))
            .collect();
        after_edges.push(Self::raw_edge(child, parent, mode));
        let (before, after) = (
            self.member_sets(&before_edges),
            self.member_sets(&after_edges),
        );
        let (mut affected, mut gained, mut lost) = (0_u32, BTreeSet::new(), BTreeSet::new());
        for (g, was) in &before {
            let now = &after[g];
            if now != was && !self.hidden(*g) {
                affected += 1;
                gained.extend(now.difference(was).cloned());
                lost.extend(was.difference(now).cloned());
            }
        }
        let count = |s: &BTreeSet<String>| u32::try_from(s.len()).unwrap_or(u32::MAX);
        Ok(NestingEffect {
            mode,
            groups_affected: affected,
            persons_gained: count(&gained),
            persons_lost: count(&lost),
        })
    }
}
