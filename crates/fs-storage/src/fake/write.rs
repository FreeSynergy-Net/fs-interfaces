//! Writing in the fake: write, create, rename, move, copy, delete, restore, changes. Never replaces, never follows a
//! link, checks the name and the right before anything changes.
//!
//! Pattern: Guard (checks first, then one change; a refused call changes nothing).

use super::access::{Actor, Mode};
use super::state::{Fault, Node, World};
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

    /// `id` or one of its ancestors is in the trash.
    pub(super) fn in_trash(&self, id: &str) -> bool {
        let mut cur = id.to_owned();
        while let Some(n) = self.nodes.get(&cur) {
            if n.trashed_from.is_some() {
                return true;
            }
            match &n.parent {
                Some(p) => cur.clone_from(p),
                None => break,
            }
        }
        false
    }

    /// The entry a link points to (only for the mutants that follow links).
    pub(super) fn deref(&self, id: &EntryId, on: bool) -> EntryId {
        match self.nodes.get(id.as_str()) {
            Some(n) if on && n.entry.kind == Kind::Link => {
                n.entry.link_target.clone().unwrap_or_else(|| id.clone())
            }
            _ => id.clone(),
        }
    }

    pub(super) fn folder_for_write(&self, a: &Actor, at: &EntryId) -> Result<&Node> {
        if at.as_str() == "trash" && self.is_bot(a) {
            return Err(Error::Forbidden {
                capability: "storage.write".to_owned(),
            });
        }
        let n = self.node_for(a, at, Mode::Write)?;
        let trashed = if a.fault == Some(Fault::WriteBelowTrashed) {
            n.trashed_from.is_some()
        } else {
            self.in_trash(at.as_str())
        };
        match n.entry.kind {
            Kind::Folder if !trashed => Ok(n),
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

    /// The folder a write goes to: the mutants `FollowLinksWrite` and `WriteIntoTrash` bend it.
    fn write_target(&self, a: &Actor, at: &EntryId) -> EntryId {
        if a.fault == Some(Fault::WriteIntoTrash) && at.as_str() == "trash" {
            return self
                .roots
                .iter()
                .find(|r| r.kind == crate::entry::RootKind::Mine && r.owner.id == a.who.id)
                .map_or_else(|| at.clone(), |r| r.id.clone());
        }
        self.deref(at, a.fault == Some(Fault::FollowLinksWrite))
    }

    /// Only the right on the target folder (a write checks it before it reads the body).
    pub(super) fn check_write_at(&self, a: &Actor, at: &EntryId) -> Result<()> {
        let at = self.write_target(a, at);
        self.folder_for_write(a, &at).map(|_| ())
    }

    pub(super) fn write_file(
        &mut self,
        a: &Actor,
        at: &EntryId,
        name: &str,
        bytes: &[u8],
        o: &WriteOpts,
    ) -> Result<Entry> {
        let at = &self.write_target(a, at);
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
        if kind != Kind::Folder && a.fault == Some(Fault::ValidityBeforeRights) {
            return Err(bad("kind", "only-folder"));
        }
        let at = &self.write_target(a, at);
        let owner = self.folder_for_write(a, at)?.entry.owner.clone();
        if kind != Kind::Folder {
            return Err(bad("kind", "only-folder"));
        }
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
        let parent = match (&n.parent, a.fault) {
            (Some(p), _) => p.clone(),
            (None, Some(Fault::RootMutable)) => String::new(),
            (None, _) => return Err(bad("id", "root")),
        };
        if self.in_trash(id.as_str()) {
            return Err(bad("id", "trashed"));
        }
        let name = if a.fault == Some(Fault::RenameReplaces) {
            self.free_name(a, &parent, name, OnConflict::KeepBoth, Some(id.as_str()))
                .and_then(|_| {
                    validate_name(name)?;
                    let old = self
                        .nodes
                        .iter()
                        .find(|(k, c)| {
                            c.parent.as_deref() == Some(parent.as_str())
                                && c.entry.name == name
                                && k.as_str() != id.as_str()
                        })
                        .map(|(k, _)| k.clone());
                    if let Some(o) = old {
                        self.nodes.remove(&o);
                    }
                    Ok(name.to_owned())
                })?
        } else {
            self.free_name(a, &parent, name, on, Some(id.as_str()))?
        };
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
}
