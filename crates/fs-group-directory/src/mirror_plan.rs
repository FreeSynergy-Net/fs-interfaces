//! The mirror plan: what Kanidm should hold (`projection`) and the steps and findings against what it holds now.
//!
//! Pattern: Strategy-free pure computation on the state (Template method: project, diff, gate by `Manage`, number
//! the findings); nothing is written.

use crate::groups_v1::shared::{ObjectId, PersonRef};
use crate::groups_v1::{
    DirectoryGroup, FindingId, FindingKind, InForce, MirrorFinding, MirrorStep, Right,
};
use crate::nesting_logic::arcs;
use crate::state::State;
use std::collections::BTreeMap;

/// A finding with the group it belongs to and whether a human may overwrite it.
pub(crate) type Raw = (Option<ObjectId>, FindingKind, bool);

/// The result of a computation.
pub(crate) struct Computed {
    pub steps: Vec<MirrorStep>,
    pub findings: Vec<MirrorFinding>,
    pub wanted: BTreeMap<ObjectId, DirectoryGroup>,
}

impl State {
    /// Whether `g` or a group it becomes a member of over arcs carries an admin role.
    fn reaches_admin(&self, g: ObjectId, pairs: &[(ObjectId, ObjectId)]) -> bool {
        let mut seen = vec![g];
        let mut at = 0;
        while at < seen.len() {
            let cur = seen[at];
            at += 1;
            if self.is_admin_group(cur) {
                return true;
            }
            for (_, v) in pairs.iter().filter(|(u, _)| *u == cur) {
                if !seen.contains(v) {
                    seen.push(*v);
                }
            }
        }
        false
    }

    /// What the directory should hold: mirrored groups with direct persons and native `nested`, plus findings.
    fn projection(&self) -> (BTreeMap<ObjectId, DirectoryGroup>, Vec<Raw>) {
        let live = |g: &ObjectId| self.groups.get(g).is_some_and(|r| r.deleted.is_none());
        let on = |g: &ObjectId| {
            self.groups
                .get(g)
                .is_some_and(|r| r.group.mirror && !r.group.sensitive)
        };
        let mut findings: Vec<Raw> = Vec::new();
        let pairs = arcs(&self.logic_edges());
        let mut wanted = BTreeMap::new();
        for (id, row) in self.groups.iter().filter(|(id, _)| live(id) && on(id)) {
            let mut members: Vec<PersonRef> = Vec::new();
            for m in self.members.iter().filter(|m| m.group == *id) {
                let unknown = self.people.status(&m.person).is_none();
                if self.member_force(m) == InForce::KindMismatch {
                    findings.push((Some(*id), FindingKind::LeftOut(m.person.clone()), false));
                    findings.push((Some(*id), FindingKind::HeldBack(*id), false));
                } else if unknown && self.is_admin_group(*id) {
                    findings.push((Some(*id), FindingKind::LeftOut(m.person.clone()), false));
                } else if !members.contains(&m.person) {
                    members.push(m.person.clone());
                }
            }
            let mut nested = Vec::new();
            for (inner, _) in pairs.iter().filter(|(_, outer)| outer == id) {
                if !on(inner) {
                    findings.push((
                        Some(*id),
                        FindingKind::NestedNotMirrored {
                            outer: *id,
                            inner: *inner,
                        },
                        false,
                    ));
                    continue;
                }
                let odd: Vec<PersonRef> = self
                    .effective_raw(*inner)
                    .into_iter()
                    .map(|e| e.person)
                    .filter(|p| self.people.status(p).is_none())
                    .collect();
                if !odd.is_empty() && self.reaches_admin(*id, &pairs) {
                    findings.extend(
                        odd.into_iter()
                            .map(|p| (Some(*inner), FindingKind::LeftOut(p), false)),
                    );
                    findings.push((Some(*inner), FindingKind::HeldBack(*inner), false));
                } else {
                    nested.push(*inner);
                }
            }
            wanted.insert(
                *id,
                DirectoryGroup {
                    group: *id,
                    name: row.group.name.clone(),
                    members,
                    nested,
                    foreign: Vec::new(),
                },
            );
        }
        (wanted, findings)
    }

    /// Steps and findings of one run against the directory as it is, `known` = the arcs the last run wrote.
    pub(crate) fn compute_plan(
        &self,
        current: &[DirectoryGroup],
        known: &[(ObjectId, ObjectId)],
    ) -> Computed {
        let (wanted, mut raw) = self.projection();
        let mut steps = Vec::new();
        for (id, want) in &wanted {
            let Some(have) = current.iter().find(|h| h.group == *id) else {
                steps.push(MirrorStep::Create { group: *id });
                if !want.members.is_empty() || !want.nested.is_empty() {
                    steps.push(MirrorStep::Update {
                        group: *id,
                        add: want.members.clone(),
                        remove: Vec::new(),
                        nest: want.nested.clone(),
                        unnest: Vec::new(),
                    });
                }
                continue;
            };
            let add: Vec<PersonRef> = want
                .members
                .iter()
                .filter(|p| !have.members.contains(p))
                .cloned()
                .collect();
            let remove: Vec<PersonRef> = have
                .members
                .iter()
                .filter(|p| !want.members.contains(p))
                .cloned()
                .collect();
            let nest: Vec<ObjectId> = want
                .nested
                .iter()
                .filter(|g| !have.nested.contains(g))
                .copied()
                .collect();
            let mut unnest = Vec::new();
            for inner in have.nested.iter().filter(|g| !want.nested.contains(g)) {
                if known.contains(&(*id, *inner)) {
                    unnest.push(*inner);
                } else {
                    raw.push((
                        Some(*id),
                        FindingKind::ForeignNesting {
                            outer: *id,
                            inner: *inner,
                        },
                        true,
                    ));
                }
            }
            raw.extend(
                have.foreign
                    .iter()
                    .map(|a| (Some(*id), FindingKind::Foreign(a.clone()), true)),
            );
            if !(add.is_empty() && remove.is_empty() && nest.is_empty() && unnest.is_empty()) {
                steps.push(MirrorStep::Update {
                    group: *id,
                    add,
                    remove,
                    nest,
                    unnest,
                });
            }
        }
        steps.extend(
            current
                .iter()
                .filter(|h| !wanted.contains_key(&h.group))
                .map(|h| MirrorStep::Remove { group: h.group }),
        );
        self.gate(&mut steps, &mut raw);
        let mut kinds: Vec<Raw> = Vec::new();
        for r in raw {
            if !kinds.iter().any(|k| k.1 == r.1) {
                kinds.push(r);
            }
        }
        let findings = kinds
            .into_iter()
            .zip(1_u32..)
            .map(|((group, kind, overwritable), n)| MirrorFinding {
                id: FindingId(n),
                group,
                kind,
                overwritable,
            })
            .collect();
        Computed {
            steps,
            findings,
            wanted,
        }
    }

    /// Whoever acts needs `Manage` at every living group the plan changes; the others stay `HeldBack`.
    fn gate(&self, steps: &mut Vec<MirrorStep>, raw: &mut Vec<Raw>) {
        let mut held = Vec::new();
        steps.retain(|s| {
            let g = match s {
                MirrorStep::Create { group }
                | MirrorStep::Update { group, .. }
                | MirrorStep::Remove { group } => *group,
            };
            let refused = self.alive(g).is_ok() && !self.may_ask(g, Right::Manage);
            if refused {
                held.push(g);
            }
            !refused
        });
        raw.extend(
            held.into_iter()
                .map(|g| (Some(g), FindingKind::HeldBack(g), false)),
        );
    }
}
