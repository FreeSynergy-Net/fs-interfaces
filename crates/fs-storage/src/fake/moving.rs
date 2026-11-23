//! Moving and copying in the fake: the rights on source AND target, cycles, owners follow the target folder.
//!
//! Pattern: Guard (checks first, then one change; a refused call changes nothing).

use super::access::{Actor, Mode};
use super::state::{Fault, Node, World};
use super::write::bad;
use crate::ctl::Ctl;
use crate::entry::{ChangeKind, Entry, Kind, OnConflict};
use crate::error::{Error, Result};
use crate::ids::EntryId;

impl World {
    fn set_owner(&mut self, id: &str, owner: &fs_types::PersonRef) {
        let kids: Vec<String> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.parent.as_deref() == Some(id))
            .map(|(k, _)| k.clone())
            .collect();
        if let Some(n) = self.nodes.get_mut(id) {
            n.entry.owner = owner.clone();
        }
        for k in kids {
            self.set_owner(&k, owner);
        }
    }

    /// The owner of the target folder; the right to write there is checked (the mutant `skip` does not).
    pub(super) fn target_owner(
        &self,
        a: &Actor,
        to: &EntryId,
        skip: Fault,
    ) -> Result<fs_types::PersonRef> {
        if a.fault == Some(skip) {
            return self
                .nodes
                .get(to.as_str())
                .map(|n| n.entry.owner.clone())
                .ok_or_else(|| Error::NotFound(to.clone()));
        }
        Ok(self.folder_for_write(a, to)?.entry.owner.clone())
    }

    pub(super) fn move_node(
        &mut self,
        a: &Actor,
        id: &EntryId,
        to: &EntryId,
        on: OnConflict,
    ) -> Result<Entry> {
        let n = self.node_for(a, id, Mode::Write)?;
        let rootish = n.parent.is_none() && a.fault != Some(Fault::RootMutable);
        let trashed = if a.fault == Some(Fault::MoveOutOfTrashed) {
            n.trashed_from.is_some()
        } else {
            self.in_trash(id.as_str())
        };
        if (rootish || trashed) && a.fault == Some(Fault::MoveValidityFirst) {
            return Err(bad("id", "root-or-trashed"));
        }
        let to = &self.write_target(a, to);
        let owner = self.target_owner(a, to, Fault::MoveIgnoresTarget)?;
        if rootish || trashed {
            return Err(bad("id", "root-or-trashed"));
        }
        let mut up = Some(to.as_str().to_owned());
        while let Some(p) = up {
            if p == id.as_str() && a.fault != Some(Fault::NoCycle) {
                return Err(Error::Cycle(id.clone()));
            }
            up = self.nodes.get(&p).and_then(|x| x.parent.clone());
        }
        let name = self.nodes[id.as_str()].entry.name.clone();
        let name = self.free_name(a, to.as_str(), &name, on, Some(id.as_str()))?;
        self.set_owner(id.as_str(), &owner);
        let n = self
            .nodes
            .get_mut(id.as_str())
            .ok_or_else(|| Error::NotFound(id.clone()))?;
        n.parent = Some(to.as_str().to_owned());
        n.entry.name = name;
        let e = n.entry.clone();
        self.log_change(id.as_str(), ChangeKind::Moved);
        Ok(e)
    }

    pub(super) fn copy_node(
        &mut self,
        a: &Actor,
        id: &EntryId,
        to: &EntryId,
        on: OnConflict,
        ctl: &Ctl,
    ) -> Result<Entry> {
        let id = &self.deref(id, a.fault == Some(Fault::CopyFollowsLink));
        let src = self.node_for(a, id, Mode::Read)?.clone();
        let to = &self.write_target(a, to);
        let owner = self.target_owner(a, to, Fault::CopyIgnoresTarget)?;
        if self.in_trash(id.as_str()) && a.fault != Some(Fault::CopyInTrash) {
            return Err(bad("id", "trashed"));
        }
        let mut up = Some(to.as_str().to_owned());
        while let Some(p) = up {
            if p == id.as_str() && a.fault != Some(Fault::NoCycle) {
                return Err(Error::Cycle(id.clone()));
            }
            up = self.nodes.get(&p).and_then(|x| x.parent.clone());
        }
        let name = self.free_name(a, to.as_str(), &src.entry.name, on, None)?;
        let new = self.copy_rec(&src, id.as_str(), to.as_str(), &name, &owner, ctl)?;
        self.log_operator_read(a, id);
        self.log_change(&new, ChangeKind::Created);
        Ok(self.nodes[&new].entry.clone())
    }

    fn copy_rec(
        &mut self,
        src: &Node,
        src_id: &str,
        to: &str,
        name: &str,
        owner: &fs_types::PersonRef,
        ctl: &Ctl,
    ) -> Result<String> {
        ctl.check()?;
        if depth_guard(self.nodes.len()) {
            return Err(Error::BackendFailed("copy loop".into()));
        }
        let id = self.next_id("n");
        let mut node = src.clone();
        node.parent = Some(to.to_owned());
        node.trashed_from = None;
        node.entry.id = EntryId(id.clone());
        name.clone_into(&mut node.entry.name);
        node.entry.owner = owner.clone();
        node.entry.modified = Some(self.now());
        self.nodes.insert(id.clone(), node);
        if src.entry.kind == Kind::Folder {
            let kids: Vec<Node> = self
                .nodes
                .iter()
                .filter(|(k, n)| {
                    n.parent.as_deref() == Some(src_id) && n.trashed_from.is_none() && **k != id
                })
                .map(|(_, n)| n.clone())
                .collect();
            for k in kids {
                let (kid, kname) = (k.entry.id.0.clone(), k.entry.name.clone());
                self.copy_rec(&k, &kid, &id, &kname, owner, ctl)?;
            }
        }
        Ok(id)
    }
}

/// A copy into itself (the `NoCycle` mutant) must not loop for ever.
fn depth_guard(nodes: usize) -> bool {
    nodes > 400
}
