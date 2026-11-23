//! The trait `StorageProvider` (ADR 0070): core `capabilities`, `roots`, `list`, `stat`, `open`; the rest `Unsupported`.
//!
//! Pattern: Repository (trait) with Null Object (not overridden methods answer `Unsupported`, as in ADR 0045).

use crate::ctl::{Blob, Ctl};
use crate::entry::{
    Change, ChangeSet, Delete, Entry, Kind, OnConflict, PageReq, ProviderState, Root, StorageCaps,
    Token, WriteOpts,
};
use crate::error::{Error, Result};
use crate::ids::{EntryId, split_path};
use async_trait::async_trait;
use fs_types::Page;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// The "not possible" of a method that is not overridden.
///
/// # Errors
/// Always `Error::Unsupported`.
pub fn not_possible<T>(capability: &str) -> Result<T> {
    Err(Error::Unsupported {
        capability: capability.to_owned(),
    })
}

/// Receiver of pushed changes (`watch`).
pub type Listener = Arc<dyn Fn(Change) + Send + Sync>;

/// A running `watch`; cancel or drop ends it.
#[derive(Clone, Debug, Default)]
pub struct Subscription {
    ended: Arc<AtomicBool>,
}

impl Subscription {
    /// A new running subscription.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// End it (the receiver must then ask again and read anew).
    pub fn end(&self) {
        self.ended.store(true, Ordering::SeqCst);
    }
    /// Whether it ended.
    #[must_use]
    pub fn has_ended(&self) -> bool {
        self.ended.load(Ordering::SeqCst)
    }
}

/// One tree of everything the asker may reach. Who asks is a property of the provider (the person on whose behalf it
/// is called), never a parameter; every call checks the rights of that person (ADR 0050, 0029), never with service
/// rights. Writes: `ByteSource` is the whole body for now; a stream form follows with the first real provider.
#[async_trait]
pub trait StorageProvider: Send + Sync {
    /// What the provider can do.
    fn capabilities(&self) -> StorageCaps;
    /// State: ready, needs login, offline, read-only.
    fn state(&self) -> ProviderState {
        ProviderState::Ready
    }
    /// "My files", one folder per group the asker belongs to, and the trash.
    async fn roots(&self) -> Result<Vec<Root>>;
    /// One level below `at`, paged. A link is never followed (`LinkNotFollowed`).
    async fn list(&self, at: &EntryId, page: PageReq) -> Result<Page<Entry>>;
    /// One entry.
    async fn stat(&self, id: &EntryId) -> Result<Entry>;
    /// The bytes of a file, readable by range. A link or folder is refused.
    async fn open(&self, id: &EntryId, ctl: &Ctl) -> Result<Box<dyn Blob>>;

    /// Write a file below `at`. Never replaces; see `OnConflict`.
    async fn write(
        &self,
        _at: &EntryId,
        _name: &str,
        _src: Vec<u8>,
        _o: WriteOpts,
        _ctl: &Ctl,
    ) -> Result<Entry> {
        not_possible("storage.write")
    }
    /// Create a folder (or link) below `at`.
    async fn create(
        &self,
        _at: &EntryId,
        _name: &str,
        _kind: Kind,
        _on: OnConflict,
    ) -> Result<Entry> {
        not_possible("storage.create")
    }
    /// Rename.
    async fn rename(&self, _id: &EntryId, _name: &str, _on: OnConflict) -> Result<Entry> {
        not_possible("storage.rename")
    }
    /// Move below another folder.
    async fn move_to(&self, _id: &EntryId, _to: &EntryId, _on: OnConflict) -> Result<Entry> {
        not_possible("storage.move")
    }
    /// Copy (a link is copied as a link, a folder with its content).
    async fn copy_to(
        &self,
        _id: &EntryId,
        _to: &EntryId,
        _on: OnConflict,
        _ctl: &Ctl,
    ) -> Result<Entry> {
        not_possible("storage.copy")
    }
    /// Into the trash or gone for good.
    async fn delete(&self, _id: &EntryId, _how: Delete) -> Result<()> {
        not_possible("storage.trash")
    }
    /// Out of the trash, to its old place or to `to`.
    async fn restore(
        &self,
        _id: &EntryId,
        _to: Option<&EntryId>,
        _on: OnConflict,
    ) -> Result<Entry> {
        not_possible("storage.restore")
    }
    /// Push changes below `at`.
    async fn watch(&self, _at: &EntryId, _l: Listener) -> Result<Subscription> {
        not_possible("storage.watch")
    }
    /// Delta since a token (`None` = from now).
    async fn changes(&self, _since: Option<&Token>) -> Result<ChangeSet> {
        not_possible("storage.delta")
    }

    /// Walk a display path below `root` by `list`. Every segment is checked first (`..`, `/`, control characters are
    /// refused, never interpreted); a link in the middle stops the walk with `LinkNotFollowed`.
    ///
    /// # Errors
    /// `InvalidName`, `NotFound`, `LinkNotFollowed`, or what `list` and `stat` answer.
    async fn resolve_path(&self, root: &EntryId, path: &str) -> Result<Entry> {
        let parts = split_path(path)?;
        let mut cur = self.stat(root).await?;
        for part in parts {
            match cur.kind {
                Kind::Link => return Err(Error::LinkNotFollowed(cur.id)),
                Kind::Folder => {}
                _ => return Err(Error::NotAFolder(cur.id)),
            }
            let mut req = PageReq::first(100);
            let found = loop {
                let page = self.list(&cur.id, req.clone()).await?;
                if let Some(e) = page.items.into_iter().find(|e| e.name == part) {
                    break Some(e);
                }
                match page.next {
                    Some(c) => req.cursor = Some(c),
                    None => break None,
                }
            };
            cur = found.ok_or_else(|| Error::NotFound(EntryId(part.to_owned())))?;
        }
        Ok(cur)
    }
}
