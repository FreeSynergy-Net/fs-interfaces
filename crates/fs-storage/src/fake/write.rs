//! Writing in the fake: write, create, rename, move, copy, delete, restore, changes. Never replaces, never follows a
//! link, checks the name and the right before anything changes.
//!
//! Pattern: Guard (checks first, then one change; a refused call changes nothing).

use super::access::{Actor, Mode};
use super::state::{Fault, Node, World};
use crate::ctl::Ctl;
use crate::entry::{Change, ChangeKind, Entry, Kind, OnConflict, WriteOpts};
use crate::error::{Error, Result};
use crate::ids::{EntryId, validate_name};
use fs_types::{MediaType, Problem};

pub(super) fn bad(field: &str, reason: &str) -> Error {
    Error::Invalid(vec![Problem {
        field: field.into(),
        reason: reason.into(),
    }])
}

/// `name (2).ext` for the n-th try.
fn numbered(name: &str, n: u32) -> String {
    match name.rfind('.').filter(|i| *i > 0) {
        Some(i) => format!("{} ({n}){}", &name[..i], &name[i..]),
        None => format!("{name} ({n})"),
    }
}

impl World {
    pub(super) fn log_change(&mut self, id: &str, kind: ChangeKind) {
        let owner = self.nodes.get(id).map(|n| n.entry.owner.clone());
        if let Some(owner) = owner {
            self.log.push((
                Change {
                    id: EntryId(id.to_owned()),
                    kind,
                },
                owner,
            ));
        }
    }

    pub(super) fn folder_for_write(&self, a: &Actor, at: &EntryId) -> Result<&Node> {
        let n = self.node_for(a, at, Mode::Write)?;
        match n.entry.kind {
            Kind::Folder if n.trashed_from.is_none() => Ok(n),
            Kind::Link => Err(Error::LinkNotFollowed(at.clone())),
            _ => Err(Error::NotAFolder(at.clone())),
        }
    }

    /// The name to use below `parent`: conflict handling without ever replacing (except the `Replace` mutant).
    pub(super) fn free_name(
        &mut self,
        a: &Actor,
        parent: &str,
        name: &str,
        on: OnConflict,
        me: Option<&str>,
    ) -> Result<String> {
        if a.fault != Some(Fault::SkipNameCheck) {
            validate_name(name)?;
        }
        let taken = |w: &World, n: &str| {
            w.nodes
                .iter()
                .find(|(id, c)| {
                    c.parent.as_deref() == Some(parent)
                        && c.trashed_from.is_none()
                        && c.entry.name == n
                        && Some(id.as_str()) != me
                })
                .map(|(id, _)| id.clone())
        };
        let Some(old) = taken(self, name) else {
            return Ok(name.to_owned());
        };
        if a.fault == Some(Fault::Replace) {
            self.nodes.remove(&old);
            return Ok(name.to_owned());
        }
        match on {
            OnConflict::Fail => Err(Error::NameConflict {
                name: name.to_owned(),
            }),
            OnConflict::KeepBoth => (2..1000)
                .map(|n| numbered(name, n))
                .find(|c| taken(self, c).is_none())
                .ok_or(Error::NameConflict {
                    name: name.to_owned(),
                }),
        }
    }

    pub(super) fn write_file(
        &mut self,
        a: &Actor,
        at: &EntryId,
        name: &str,
        bytes: &[u8],
        o: &WriteOpts,
    ) -> Result<Entry> {
        let owner = self.folder_for_write(a, at)?.entry.owner.clone();
        let name = self.free_name(a, at.as_str(), name, o.on_conflict, None)?;
        let media = o
            .media_type
            .clone()
            .unwrap_or(MediaType("application/octet-stream".into()));
        let id = self.add_file(
            at.as_str(),
            &name,
            &media.0,
            bytes,
            &owner,
            o.generator.as_deref(),
        );
        self.log_change(&id, ChangeKind::Created);
        Ok(self.nodes[&id].entry.clone())
    }

    pub(super) fn create_folder(
        &mut self,
        a: &Actor,
        at: &EntryId,
        name: &str,
        kind: Kind,
        on: OnConflict,
    ) -> Result<Entry> {
        if kind != Kind::Folder {
            return Err(bad("kind", "only-folder"));
        }
        let owner = self.folder_for_write(a, at)?.entry.owner.clone();
        let name = self.free_name(a, at.as_str(), name, on, None)?;
        let id = self.next_id("n");
        self.add_folder(&id, Some(at.as_str()), &name, &owner);
        self.log_change(&id, ChangeKind::Created);
        Ok(self.nodes[&id].entry.clone())
    }

    pub(super) fn rename_node(
        &mut self,
        a: &Actor,
        id: &EntryId,
        name: &str,
        on: OnConflict,
    ) -> Result<Entry> {
        let n = self.node_for(a, id, Mode::Write)?;
        let parent = n.parent.clone().ok_or_else(|| bad("id", "root"))?;
        let name = self.free_name(a, &parent, name, on, Some(id.as_str()))?;
        let t = self.now();
        let n = self
            .nodes
            .get_mut(id.as_str())
            .ok_or_else(|| Error::NotFound(id.clone()))?;
        n.entry.name = name;
        n.entry.modified = Some(t);
        let e = n.entry.clone();
        self.log_change(id.as_str(), ChangeKind::Updated);
        Ok(e)
    }

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
        if n.parent.is_none() || n.trashed_from.is_some() {
            return Err(bad("id", "root-or-trashed"));
        }
        let owner = self.target_owner(a, to, Fault::MoveIgnoresTarget)?;
        let mut up = Some(to.as_str().to_owned());
        while let Some(p) = up {
            if p == id.as_str() {
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
        let src = self.node_for(a, id, Mode::Read)?.clone();
        let owner = self.target_owner(a, to, Fault::CopyIgnoresTarget)?;
        let mut up = Some(to.as_str().to_owned());
        while let Some(p) = up {
            if p == id.as_str() {
                return Err(Error::Cycle(id.clone()));
            }
            up = self.nodes.get(&p).and_then(|x| x.parent.clone());
        }
        let name = self.free_name(a, to.as_str(), &src.entry.name, on, None)?;
        let new = self.copy_rec(&src, id.as_str(), to.as_str(), &name, &owner, ctl)?;
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
