//! `FakeMirror`: the Kanidm side in memory (`GroupDirectory` of the locked folder), with the by-hand changes a real
//! directory has: foreign accounts, a nesting laid by hand, accounts that map to no person, a directory that is down.
//!
//! Pattern: Fake (a test double with working behaviour) behind the consumed `GroupDirectory` port.

use crate::groups_v1::shared::{ObjectId, PersonRef};
use crate::groups_v1::{
    Applied, DirectoryAccount, DirectoryError, DirectoryGroup, DirectoryId, GroupDirectory,
};
use crate::ids::reference_of;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

#[derive(Debug, Default)]
struct Dir {
    groups: BTreeMap<ObjectId, DirectoryGroup>,
    /// Persons without an account: they stay out and are reported by `apply`.
    unmapped: Vec<PersonRef>,
    down: bool,
}

/// An in-memory directory.
#[derive(Debug, Default)]
pub struct FakeMirror {
    dir: Mutex<Dir>,
}

type Reply<T> = std::result::Result<T, DirectoryError>;

impl FakeMirror {
    /// An empty, reachable directory.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, Dir> {
        self.dir
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Make the directory unreachable (`true`) or reachable again.
    pub fn set_down(&self, down: bool) {
        self.lock().down = down;
    }

    /// A person has no account in the directory.
    pub fn set_unmapped(&self, person: &PersonRef) {
        self.lock().unmapped.push(person.clone());
    }

    /// Put a group in as it is (state before the first run, old data).
    pub fn seed(&self, group: DirectoryGroup) {
        self.lock().groups.insert(group.group, group);
    }

    /// An account added by hand in the directory.
    pub fn add_foreign(&self, group: ObjectId, account: &str) {
        let mut d = self.lock();
        if let Some(g) = d.groups.get_mut(&group) {
            g.foreign.push(DirectoryAccount(account.to_owned()));
        }
    }

    /// A nesting laid by hand: `inner` becomes a member of `outer`.
    pub fn nest_by_hand(&self, outer: ObjectId, inner: ObjectId) {
        let mut d = self.lock();
        if let Some(g) = d.groups.get_mut(&outer) {
            g.nested.push(inner);
        }
    }

    /// A person added by hand to a group in the directory.
    pub fn add_member_by_hand(&self, group: ObjectId, person: &PersonRef) {
        let mut d = self.lock();
        if let Some(g) = d.groups.get_mut(&group) {
            g.members.push(person.clone());
        }
    }

    /// The group as the directory holds it now.
    #[must_use]
    pub fn group(&self, id: ObjectId) -> Option<DirectoryGroup> {
        self.lock().groups.get(&id).cloned()
    }
}

#[async_trait]
impl GroupDirectory for FakeMirror {
    fn directory_id(&self) -> DirectoryId {
        DirectoryId(reference_of(crate::ids::object_id(9_000_000)))
    }

    async fn list(&self) -> Reply<Vec<DirectoryGroup>> {
        let d = self.lock();
        if d.down {
            return Err(DirectoryError::Unreachable);
        }
        Ok(d.groups.values().cloned().collect())
    }

    async fn read(&self, group: ObjectId) -> Reply<Option<DirectoryGroup>> {
        let d = self.lock();
        if d.down {
            return Err(DirectoryError::Unreachable);
        }
        Ok(d.groups.get(&group).cloned())
    }

    async fn apply(&self, group: &DirectoryGroup) -> Reply<Applied> {
        let mut d = self.lock();
        if d.down {
            return Err(DirectoryError::Unreachable);
        }
        let (kept, left): (Vec<PersonRef>, Vec<PersonRef>) = group
            .members
            .iter()
            .cloned()
            .partition(|p| !d.unmapped.contains(p));
        d.groups.insert(
            group.group,
            DirectoryGroup {
                members: kept,
                ..group.clone()
            },
        );
        Ok(Applied { unmapped: left })
    }

    async fn remove(&self, group: ObjectId) -> Reply<()> {
        let mut d = self.lock();
        if d.down {
            return Err(DirectoryError::Unreachable);
        }
        d.groups.remove(&group);
        Ok(())
    }
}
