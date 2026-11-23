//! Trash and delta in the fake: delete (trash or purge), restore, changes since a token.
//!
//! Pattern: Guard (checks first, then one change; a refused call changes nothing).

use super::access::{Actor, Mode};
use super::state::{Fault, World};
use super::write::bad;
use crate::entry::{ChangeKind, ChangeSet, Delete, Entry, OnConflict, Token};
use crate::error::{Error, Result};
use crate::ids::EntryId;

impl World {
    pub(super) fn delete_node(&mut self, a: &Actor, id: &EntryId, how: Delete) -> Result<()> {
        let id = &self.deref(id, a.fault == Some(Fault::DeleteFollowsLink));
        let n = self.node_for(a, id, Mode::Write)?;
        let parent = match (&n.parent, a.fault) {
            (Some(p), _) => p.clone(),
            (None, Some(Fault::RootMutable)) => String::new(),
            (None, _) => return Err(bad("id", "root")),
        };
        let trashed = n.trashed_from.is_some();
        let how = if a.fault == Some(Fault::PurgeOnTrash) {
            Delete::Purge
        } else {
            how
        };
        if how == Delete::Trash {
            if trashed {
                return Err(Error::NotFound(id.clone()));
            }
            if let Some(n) = self.nodes.get_mut(id.as_str()) {
                n.trashed_from = Some(parent);
            }
            self.log_change(id.as_str(), ChangeKind::Trashed);
            return Ok(());
        }
        self.log_change(id.as_str(), ChangeKind::Purged);
        let mut gone = vec![id.as_str().to_owned()];
        while let Some(g) = gone.pop() {
            gone.extend(
                self.nodes
                    .iter()
                    .filter(|(_, n)| n.parent.as_deref() == Some(g.as_str()))
                    .map(|(k, _)| k.clone()),
            );
            self.nodes.remove(&g);
        }
        Ok(())
    }

    pub(super) fn restore_node(
        &mut self,
        a: &Actor,
        id: &EntryId,
        to: Option<&EntryId>,
        on: OnConflict,
    ) -> Result<Entry> {
        let mode = if a.fault == Some(Fault::RestoreNotOwner) {
            Mode::Read
        } else {
            Mode::Write
        };
        let n = self.node_for(a, id, mode)?;
        let from = n
            .trashed_from
            .clone()
            .ok_or_else(|| Error::NotFound(id.clone()))?;
        let dest = to.cloned().unwrap_or(EntryId(from));
        if a.fault == Some(Fault::RestoreIgnoresTarget) {
            self.nodes
                .get(dest.as_str())
                .ok_or_else(|| bad("to", "gone"))?;
        } else {
            self.folder_for_write(a, &dest).map_err(|e| {
                if matches!(e, Error::NotFound(_)) {
                    bad("to", "gone")
                } else {
                    e
                }
            })?;
        }
        let name = self.nodes[id.as_str()].entry.name.clone();
        let name = self.free_name(a, dest.as_str(), &name, on, Some(id.as_str()))?;
        let n = self
            .nodes
            .get_mut(id.as_str())
            .ok_or_else(|| Error::NotFound(id.clone()))?;
        n.trashed_from = None;
        n.parent = Some(dest.as_str().to_owned());
        n.entry.name = name;
        let e = n.entry.clone();
        self.log_change(id.as_str(), ChangeKind::Restored);
        Ok(e)
    }

    pub(super) fn changes_since(&self, a: &Actor, since: Option<&Token>) -> Result<ChangeSet> {
        if self.is_bot(a) {
            return Err(Error::Forbidden {
                capability: "storage.read".to_owned(),
            });
        }
        let from = match since {
            None => self.log.len(),
            Some(t) => {
                t.0.parse::<usize>()
                    .ok()
                    .filter(|i| *i <= self.log.len())
                    .ok_or(Error::Changed)?
            }
        };
        let changes = self.log[from..]
            .iter()
            .filter(|(_, o)| a.fault == Some(Fault::ChangesLeak) || self.allowed(a, o, Mode::Read))
            .map(|(c, _)| c.clone())
            .collect();
        Ok(ChangeSet {
            changes,
            token: Token(self.log.len().to_string()),
        })
    }
}
