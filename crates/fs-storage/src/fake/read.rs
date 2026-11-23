//! Reading in the fake: roots, list (one level, paged), stat, open. Rights are checked on every id.
//!
//! Pattern: Guard (every id passes `node_for` before it is used).

use super::access::{Actor, Mode};
use super::state::{Fault, Node, World};
use crate::ctl::Blob;
use crate::entry::{Entry, Kind, PageReq, Root, RootKind, Sort, Usage};
use crate::error::{Error, Result};
use crate::ids::EntryId;
use async_trait::async_trait;
use fs_objects::v5::files::MemoryFiles;
use fs_types::{FileRef, PageCursor, Problem};
use std::sync::Arc;

pub(super) const TRASH: &str = "trash";

pub(crate) struct MemBlob {
    pub entry: Entry,
    pub data: Vec<u8>,
}

#[async_trait]
impl Blob for MemBlob {
    fn entry(&self) -> &Entry {
        &self.entry
    }
    async fn read(&self, offset: u64, length: u64) -> Result<Vec<u8>> {
        let len = self.data.len() as u64;
        let from = offset.min(len);
        let to = offset.saturating_add(length).min(len);
        Ok(
            self.data[usize::try_from(from).unwrap_or(0)..usize::try_from(to).unwrap_or(0)]
                .to_vec(),
        )
    }
}

impl World {
    /// The node of `id` if the asker may read (or write) it.
    pub(super) fn node_for(&self, a: &Actor, id: &EntryId, mode: Mode) -> Result<&Node> {
        let n = self
            .nodes
            .get(id.as_str())
            .ok_or_else(|| Error::NotFound(id.clone()))?;
        if self.allowed(a, &n.entry.owner, mode) {
            Ok(n)
        } else {
            let capability = if mode == Mode::Write {
                "storage.write"
            } else {
                "storage.read"
            };
            Err(Error::Forbidden {
                capability: capability.to_owned(),
            })
        }
    }

    /// The entry as shown (a fault may spoil the sha256).
    pub(super) fn present(&self, a: &Actor, n: &Node) -> Entry {
        let mut e = n.entry.clone();
        if let (Some(Fault::WrongSha), Some(f)) = (a.fault, e.file_ref.as_mut()) {
            f.sha256[0] ^= 0xff;
        }
        e
    }

    pub(super) fn usage_of(&self, root: &str) -> Usage {
        let mut u = Usage { bytes: 0, items: 0 };
        let mut stack = vec![root.to_owned()];
        while let Some(p) = stack.pop() {
            for (k, n) in self.nodes.iter().filter(|(_, n)| {
                n.parent.as_deref() == Some(p.as_str()) && n.trashed_from.is_none()
            }) {
                if n.entry.kind == Kind::Folder {
                    stack.push(k.clone());
                } else if n.entry.kind == Kind::File {
                    u.bytes += n.entry.size.unwrap_or(0);
                    u.items += 1;
                }
            }
        }
        u
    }

    /// Own "My files", the group folders of the asker's groups, the trash. An admin also gets the private roots of
    /// everyone, with `usage` only (`stat` and `list` below them answer Forbidden) and the group folders (structure).
    pub(super) fn roots_of(&self, a: &Actor) -> Vec<Root> {
        let bot = self.bots.iter().any(|p| p.id == a.who.id) && a.fault != Some(Fault::BotAllowed);
        let mut out: Vec<Root> = self
            .roots
            .iter()
            .filter(|r| match r.kind {
                RootKind::Mine => r.owner.id == a.who.id || self.is_admin(a),
                RootKind::Group => self.allowed(a, &r.owner, Mode::Structure),
                _ => false,
            })
            .filter(|_| !bot)
            .cloned()
            .map(|mut r| {
                r.usage = Some(self.usage_of(r.id.as_str()));
                r
            })
            .collect();
        if !bot {
            out.push(Root {
                id: EntryId(TRASH.into()),
                name: "Papierkorb".into(),
                kind: RootKind::Trash,
                owner: a.who.clone(),
                usage: None,
            });
        }
        out
    }

    pub(super) fn stat_of(&self, a: &Actor, id: &EntryId) -> Result<Entry> {
        if id.as_str() == TRASH {
            let mut e = self.nodes["mine-anna"].entry.clone();
            e.id = id.clone();
            e.name = "Papierkorb".into();
            e.owner = a.who.clone();
            return Ok(e);
        }
        self.node_for(a, id, Mode::Structure)
            .map(|n| self.present(a, n))
    }

    pub(super) fn list_of(
        &self,
        a: &Actor,
        at: &EntryId,
        page: &PageReq,
    ) -> Result<(Vec<Entry>, Option<PageCursor>)> {
        if page.size == 0 || page.size > 1000 {
            return Err(Error::Invalid(vec![Problem {
                field: "size".into(),
                reason: "range".into(),
            }]));
        }
        let mut dir = at.clone();
        let in_trash = at.as_str() == TRASH;
        if !in_trash {
            let n = self.node_for(a, at, Mode::Structure)?;
            match (n.entry.kind, &n.entry.link_target) {
                (Kind::Folder, _) => {}
                (Kind::Link, Some(t)) if a.fault == Some(Fault::FollowLinks) => dir = t.clone(),
                (Kind::Link, _) => return Err(Error::LinkNotFollowed(at.clone())),
                _ => return Err(Error::NotAFolder(at.clone())),
            }
        }
        let mut items: Vec<&Node> = self
            .nodes
            .values()
            .filter(|n| {
                if in_trash {
                    n.trashed_from.is_some()
                        && (a.fault == Some(Fault::TrashLeak)
                            || self.allowed(a, &n.entry.owner, Mode::Structure))
                } else {
                    n.trashed_from.is_none()
                        && n.parent.as_deref() == Some(dir.as_str())
                        && self.allowed(a, &n.entry.owner, Mode::Structure)
                }
            })
            .collect();
        match page.sort {
            Sort::Name => items.sort_by(|x, y| {
                (x.entry.kind != Kind::Folder, &x.entry.name)
                    .cmp(&(y.entry.kind != Kind::Folder, &y.entry.name))
            }),
            Sort::Modified => items.sort_by_key(|x| std::cmp::Reverse(x.entry.modified)),
            Sort::Size => items.sort_by_key(|x| std::cmp::Reverse(x.entry.size)),
        }
        let start = match &page.cursor {
            None => 0,
            Some(c) => c.0.parse::<usize>().map_err(|_| {
                Error::Invalid(vec![Problem {
                    field: "cursor".into(),
                    reason: "bad".into(),
                }])
            })?,
        };
        let end = start.saturating_add(page.size).min(items.len());
        let out = items
            .get(start..end)
            .unwrap_or(&[])
            .iter()
            .map(|n| self.present(a, n))
            .collect();
        Ok((
            out,
            (end < items.len()).then(|| PageCursor(end.to_string())),
        ))
    }

    /// The entry and the reference to read through the `FileStore` (after the lock is released).
    pub(super) fn open_of(
        &self,
        a: &Actor,
        id: &EntryId,
    ) -> Result<(Entry, FileRef, Arc<MemoryFiles>)> {
        let n = self.node_for(a, id, Mode::Read)?;
        match (n.entry.kind, &n.entry.file_ref) {
            (Kind::File, Some(f)) => Ok((self.present(a, n), f.clone(), Arc::clone(&self.files))),
            (Kind::Link, _) => Err(Error::LinkNotFollowed(id.clone())),
            _ => Err(Error::Invalid(vec![Problem {
                field: "id".into(),
                reason: "not-a-file".into(),
            }])),
        }
    }
}
