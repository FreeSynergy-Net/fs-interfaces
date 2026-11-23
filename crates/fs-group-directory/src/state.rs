//! The state of a `FakeGroups`: tables as vectors, the clock, the log, the trash bags; plus the helpers every
//! operation shares (ids, clock, log, lookups).
//!
//! Pattern: Repository state (plain tables behind one lock; every operation is a sync method on this struct, the
//! async trait impl only calls them).

use crate::groups_v1::shared::{ObjectId, PersonRef, Revision, Utc};
use crate::groups_v1::{
    Change, ChangeKind, Error, Grant, Group, HistoryEntry, HistoryKind, Invitation, Membership,
    Nesting, NestingRequest, Result,
};
use crate::ids::{no_group, object_id};
use crate::mirror_state::MirrorLog;
use crate::nesting_logic::Edge;
use crate::people::FakePeople;
use std::collections::BTreeMap;
use std::sync::Arc;

/// How long a deleted group stays restorable: 30 days.
pub(crate) const TRASH_MS: i64 = 30 * 24 * 60 * 60 * 1000;
/// Every write takes one second on the fake's own clock.
pub(crate) const STEP_MS: i64 = 1000;

/// One group with its trash mark.
#[derive(Clone, Debug)]
pub(crate) struct Row {
    pub group: Group,
    /// `Some(deadline)` while in the trash.
    pub deleted: Option<Utc>,
    /// The revision at the moment of the deletion (a changed one makes `restore` say `Changed`).
    pub rev_at_delete: Revision,
}

/// What `delete` took along, until `restore` or the deadline.
#[derive(Clone, Debug, Default)]
pub(crate) struct Bag {
    pub members: Vec<Membership>,
    pub invites: Vec<Invitation>,
    pub grants: Vec<Grant>,
    pub edges: Vec<Nesting>,
}

/// All tables of one fake.
pub(crate) struct State {
    pub now: Utc,
    pub n: u64,
    pub seq: u64,
    pub asker: PersonRef,
    pub installer: Option<PersonRef>,
    pub people: Arc<FakePeople>,
    pub groups: BTreeMap<ObjectId, Row>,
    pub members: Vec<Membership>,
    pub invites: Vec<Invitation>,
    pub grants: Vec<Grant>,
    pub edges: Vec<Nesting>,
    pub requests: Vec<NestingRequest>,
    pub bags: BTreeMap<ObjectId, Bag>,
    pub history: Vec<(u64, ObjectId, HistoryEntry)>,
    pub changes: Vec<(u64, Change)>,
    pub mirror: MirrorLog,
    pub v2: crate::v2_state::V2State,
}

impl State {
    pub(crate) fn new(asker: PersonRef, now: Utc, people: Arc<FakePeople>) -> Self {
        Self {
            now,
            n: 0,
            seq: 0,
            asker,
            installer: None,
            people,
            groups: BTreeMap::new(),
            members: Vec::new(),
            invites: Vec::new(),
            grants: Vec::new(),
            edges: Vec::new(),
            requests: Vec::new(),
            bags: BTreeMap::new(),
            history: Vec::new(),
            changes: Vec::new(),
            mirror: MirrorLog::default(),
            v2: crate::v2_state::V2State::default(),
        }
    }

    /// A fresh id (above every id put in raw).
    pub(crate) fn fresh(&mut self) -> ObjectId {
        self.n += 1;
        object_id(self.n)
    }

    /// A write starts: the clock moves one step.
    pub(crate) fn tick(&mut self) -> Utc {
        self.now = Utc::from_millis(self.now.millis() + STEP_MS);
        self.now
    }

    /// Everything past its trash deadline is gone for good.
    pub(crate) fn purge(&mut self) {
        let now = self.now;
        let gone: Vec<ObjectId> = self
            .groups
            .iter()
            .filter(|(_, r)| r.deleted.is_some_and(|until| until <= now))
            .map(|(id, _)| *id)
            .collect();
        for id in gone {
            self.groups.remove(&id);
            self.bags.remove(&id);
            self.requests.retain(|r| r.child != id && r.parent != id);
            self.history.retain(|(_, g, _)| *g != id);
            self.v2.addons.retain(|a| a.group != id);
            self.v2.interventions.retain(|i| i.group != id);
        }
    }

    /// Write a log entry to the history of `group`.
    pub(crate) fn log(&mut self, group: ObjectId, kind: HistoryKind) {
        self.seq += 1;
        let entry = HistoryEntry {
            at: self.now,
            actor: self.asker.clone(),
            kind: kind.clone(),
        };
        self.history.push((self.seq, group, entry));
        self.note_intervention(group, kind);
    }

    /// Write a change-stream entry.
    pub(crate) fn change(&mut self, kind: ChangeKind) {
        self.seq += 1;
        self.changes.push((self.seq, Change { at: self.now, kind }));
    }

    /// The group, living or in the trash (no visibility rule).
    pub(crate) fn row(&self, id: ObjectId) -> Option<&Row> {
        self.groups.get(&id)
    }

    /// The living group; unknown, deleted: `NotFound` (no visibility rule; see `visible`).
    pub(crate) fn alive(&self, id: ObjectId) -> Result<&Group> {
        self.groups
            .get(&id)
            .filter(|r| r.deleted.is_none())
            .map(|r| &r.group)
            .ok_or_else(|| no_group(id))
    }

    /// The living group, `NotFound` when it is unknown, deleted or hidden from the asker. EVERY method that takes
    /// a group id calls this first (the AI visibility rule: nothing is answered before it).
    pub(crate) fn visible(&self, id: ObjectId) -> Result<&Group> {
        let group = self.alive(id)?;
        if self.hidden(id) {
            Err(no_group(id))
        } else {
            Ok(group)
        }
    }

    /// The confirmed edges as the logic sees them.
    pub(crate) fn logic_edges(&self) -> Vec<Edge<ObjectId>> {
        self.edges.iter().map(Self::logic_edge).collect()
    }

    /// One edge as the logic sees it.
    pub(crate) fn logic_edge(e: &Nesting) -> Edge<ObjectId> {
        Self::raw_edge(e.child, e.parent, e.mode)
    }

    /// The edge as stored, mode as the logic mode (for checks of NEW edges: no old-data rule).
    pub(crate) fn raw_edge(
        child: ObjectId,
        parent: ObjectId,
        mode: crate::groups_v1::NestingMode,
    ) -> Edge<ObjectId> {
        Edge {
            child,
            parent,
            mode: crate::bridge::mode_in(mode),
        }
    }

    /// `Changed` when `expected` is not the current revision.
    pub(crate) fn cas(current: Revision, expected: Revision) -> Result<()> {
        if current == expected {
            Ok(())
        } else {
            Err(Error::Changed { current })
        }
    }
}
