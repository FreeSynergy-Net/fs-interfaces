//! `impl StorageProvider for FakeStorage`: locks the world, builds the `Actor`, calls the world's methods.
//!
//! Pattern: Adapter (thin; the logic is in `read.rs` and `write.rs`).

use super::access::Actor;
use super::read::MemBlob;
use super::state::{FakeStorage, Fault};
use crate::ctl::{Blob, ByteSource, Ctl};
use crate::entry::{
    ChangeSet, Delete, Entry, Kind, OnConflict, PageReq, Root, StorageCaps, Token, WriteOpts,
};
use crate::error::{Error, Result};
use crate::ids::EntryId;
use crate::provider::StorageProvider;
use async_trait::async_trait;
use fs_types::Page;

/// Chunk size of a write body and the limit of the fake.
const CHUNK: usize = 4;
const MAX_BYTES: usize = 1 << 20;

impl FakeStorage {
    fn run<T>(&self, f: impl FnOnce(&mut super::state::World, &Actor) -> Result<T>) -> Result<T> {
        let a = Actor {
            who: &self.asker,
            fault: self.fault,
        };
        let mut w = self
            .world
            .lock()
            .map_err(|_| Error::BackendFailed("lock".into()))?;
        f(&mut w, &a)
    }
}

#[async_trait]
impl StorageProvider for FakeStorage {
    fn capabilities(&self) -> StorageCaps {
        StorageCaps {
            create: true,
            write: true,
            rename: true,
            move_to: true,
            copy: true,
            trash: true,
            restore: true,
            range: true,
            watch: false,
            delta_since: true,
            scope: true,
        }
    }
    async fn roots(&self) -> Result<Vec<Root>> {
        self.run(|w, a| Ok(w.roots_of(a)))
    }
    async fn list(&self, at: &EntryId, page: PageReq) -> Result<Page<Entry>> {
        self.run(|w, a| {
            w.list_of(a, at, &page)
                .map(|(items, next)| Page { items, next })
        })
    }
    async fn stat(&self, id: &EntryId) -> Result<Entry> {
        self.run(|w, a| w.stat_of(a, id))
    }
    async fn open(&self, id: &EntryId, ctl: &Ctl) -> Result<Box<dyn Blob>> {
        ctl.check()?;
        let (entry, data) = self.run(|w, a| w.open_of(a, id))?;
        Ok(Box::new(MemBlob {
            entry,
            data,
            bad_range: self.fault == Some(Fault::BadRange),
        }))
    }
    async fn write(
        &self,
        at: &EntryId,
        name: &str,
        src: &mut dyn ByteSource,
        o: WriteOpts,
        ctl: &Ctl,
    ) -> Result<Entry> {
        let mut bytes = Vec::new();
        loop {
            if self.fault != Some(Fault::NoCancel) {
                ctl.check()?;
            }
            let chunk = src.next(CHUNK).await?;
            if chunk.is_empty() {
                break;
            }
            if bytes.len() + chunk.len() > MAX_BYTES {
                return Err(Error::TooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        self.run(|w, a| w.write_file(a, at, name, &bytes, &o))
    }
    async fn create(&self, at: &EntryId, name: &str, kind: Kind, on: OnConflict) -> Result<Entry> {
        self.run(|w, a| w.create_folder(a, at, name, kind, on))
    }
    async fn rename(&self, id: &EntryId, name: &str, on: OnConflict) -> Result<Entry> {
        self.run(|w, a| w.rename_node(a, id, name, on))
    }
    async fn move_to(&self, id: &EntryId, to: &EntryId, on: OnConflict) -> Result<Entry> {
        self.run(|w, a| w.move_node(a, id, to, on))
    }
    async fn copy_to(
        &self,
        id: &EntryId,
        to: &EntryId,
        on: OnConflict,
        ctl: &Ctl,
    ) -> Result<Entry> {
        self.run(|w, a| w.copy_node(a, id, to, on, ctl))
    }
    async fn delete(&self, id: &EntryId, how: Delete) -> Result<()> {
        self.run(|w, a| w.delete_node(a, id, how))
    }
    async fn restore(&self, id: &EntryId, to: Option<&EntryId>, on: OnConflict) -> Result<Entry> {
        self.run(|w, a| w.restore_node(a, id, to, on))
    }
    async fn scope(&self, generator: &str, page: PageReq) -> Result<Page<Entry>> {
        self.run(|w, a| {
            w.scope_of(a, generator, &page)
                .map(|(items, next)| Page { items, next })
        })
    }
    async fn resolve_path(&self, root: &EntryId, path: &str) -> Result<Entry> {
        if self.fault == Some(Fault::PathTraversal) {
            return self.run(|w, a| w.naive_path(a, root, path));
        }
        crate::provider::resolve_by_list(self, root, path).await
    }
    async fn changes(&self, since: Option<&Token>) -> Result<ChangeSet> {
        self.run(|w, a| w.changes_since(a, since))
    }
}
