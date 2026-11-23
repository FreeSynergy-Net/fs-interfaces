//! `impl StorageProvider for FakeStorage`: locks the world, builds the `Actor`, calls the world's methods.
//!
//! Pattern: Adapter (thin; the logic is in `read.rs` and `write.rs`).

use super::access::Actor;
use super::read::MemBlob;
use super::state::{FakeStorage, Fault};
use crate::ctl::{Blob, Ctl};
use crate::entry::{
    ChangeSet, Delete, Entry, Kind, OnConflict, PageReq, Root, StorageCaps, Token, WriteOpts,
};
use crate::error::{Error, Result};
use crate::ids::EntryId;
use crate::provider::StorageProvider;
use async_trait::async_trait;
use fs_objects::v5::files::FileStore;
use fs_types::Page;

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
        let (entry, f, store) = self.run(|w, a| w.open_of(a, id))?;
        let data = store
            .read(&f)
            .await
            .map_err(|e| Error::BackendFailed(e.to_string()))?;
        Ok(Box::new(MemBlob { entry, data }))
    }
    async fn write(
        &self,
        at: &EntryId,
        name: &str,
        src: Vec<u8>,
        o: WriteOpts,
        ctl: &Ctl,
    ) -> Result<Entry> {
        if self.fault != Some(Fault::NoCancel) {
            ctl.check()?;
        }
        self.run(|w, a| w.write_file(a, at, name, src, &o))
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
    async fn changes(&self, since: Option<&Token>) -> Result<ChangeSet> {
        self.run(|w, a| w.changes_since(a, since))
    }
}
