//! The trash: `delete` takes a group with everything that hangs on it, `restore` brings it back by the rules.
//!
//! Pattern: Memento (the bag holds what the deletion took; `restore` replays it through the same checks as a write).

use crate::groups_v1::shared::{ObjectId, Revision, Utc};
use crate::groups_v1::{
    ChangeKind, Deleted, Error, Group, HistoryKind, RequestState, Result, Right,
};
use crate::ids::no_group;
use crate::state::{Bag, State, TRASH_MS};

impl State {
    pub(crate) fn delete(&mut self, id: ObjectId, expected: Revision) -> Result<Deleted> {
        let old = self.visible(id)?.clone();
        self.need(id, Right::Manage)?;
        Self::cas(old.revision, expected)?;
        let now = self.tick();
        let until = Utc::from_millis(now.millis() + TRASH_MS);
        let bag = Bag {
            members: take(&mut self.members, |m| m.group == id),
            invites: take(&mut self.invites, |i| i.group == id),
            grants: take(&mut self.grants, |g| g.group == id),
            edges: take(&mut self.edges, |e| e.child == id || e.parent == id),
        };
        for e in &bag.edges {
            let other = if e.child == id { e.parent } else { e.child };
            self.log(other, HistoryKind::NestingRemoved { other: id });
            self.change(ChangeKind::Nesting {
                child: e.child,
                parent: e.parent,
            });
        }
        for r in self
            .requests
            .iter_mut()
            .filter(|r| r.state == RequestState::Pending && (r.child == id || r.parent == id))
        {
            r.state = RequestState::Withdrawn;
            r.revision = Revision(r.revision.0 + 1);
        }
        self.bags.insert(id, bag);
        if let Some(row) = self.groups.get_mut(&id) {
            row.deleted = Some(until);
            row.rev_at_delete = row.group.revision;
        }
        self.log(id, HistoryKind::Deleted);
        self.change(ChangeKind::Deleted(id));
        Ok(Deleted {
            id,
            restorable_until: until,
        })
    }

    pub(crate) fn restore(&mut self, id: ObjectId) -> Result<Group> {
        let (deleted, rev_at, current) = match self.groups.get(&id) {
            Some(r) => (r.deleted.is_some(), r.rev_at_delete, r.group.revision),
            None => return Err(Error::NotRestorable(id)),
        };
        if self.hidden(id) {
            return Err(no_group(id));
        }
        let bag = self
            .bags
            .get(&id)
            .cloned()
            .filter(|_| deleted)
            .ok_or(Error::NotRestorable(id))?;
        let base = self.grants.len();
        self.grants.extend(bag.grants.clone());
        let allowed = self.may_ask(id, Right::Manage);
        self.grants.truncate(base);
        if !allowed {
            return Err(Error::Forbidden {
                group: id,
                right: Right::Manage,
            });
        }
        if current != rev_at {
            return Err(Error::Changed { current });
        }
        let now = self.tick();
        self.bags.remove(&id);
        self.members.extend(bag.members);
        self.invites.extend(bag.invites);
        self.grants.extend(bag.grants);
        if let Some(row) = self.groups.get_mut(&id) {
            row.deleted = None;
            row.group.revision = Revision(row.group.revision.0 + 1);
            row.group.updated = now;
        }
        for e in bag.edges {
            let other = if e.child == id { e.parent } else { e.child };
            let fits = self.alive(other).is_ok()
                && self.check_edge(e.child, e.parent, e.mode, None).is_ok();
            if fits {
                self.edges.push(e);
            } else {
                self.log(id, HistoryKind::NestingRemoved { other });
                self.log(other, HistoryKind::NestingRemoved { other: id });
            }
        }
        self.log(id, HistoryKind::Restored);
        self.change(ChangeKind::Restored(id));
        Ok(self.alive(id)?.clone())
    }

    /// Simulate a concurrent writer that changed the deleted group (the fixture's mutator).
    pub(crate) fn touch_deleted(&mut self, id: ObjectId) -> bool {
        match self.groups.get_mut(&id).filter(|r| r.deleted.is_some()) {
            Some(row) => {
                row.group.revision = Revision(row.group.revision.0 + 1);
                true
            }
            None => false,
        }
    }
}

/// Take the rows that match out of a table.
fn take<T>(rows: &mut Vec<T>, pick: impl Fn(&T) -> bool) -> Vec<T> {
    let (taken, kept): (Vec<T>, Vec<T>) = std::mem::take(rows).into_iter().partition(|r| pick(r));
    *rows = kept;
    taken
}
