//! `FakeLocal`: a foreign tree (home folder or USB) as an import source: it reads, it never saves.
//!
//! Pattern: Fake of a read-only provider (Null Object for every write: the default `Unsupported`).
//!
//! ADR 0070 point 3 as corrected 23.11.2026: `local`, `S3` and `WebDAV` are import sources and export targets, never the
//! place where "save" goes. Their entries have no `file_ref` before the import (no checksum is known).

use super::read::MemBlob;
use super::sample::sample;
use crate::ctl::{Blob, Ctl};
use crate::entry::{Entry, Kind, PageReq, Root, RootKind, StorageCaps};
use crate::error::{Error, Result};
use crate::ids::EntryId;
use crate::provider::StorageProvider;
use async_trait::async_trait;
use fs_types::{MediaType, Page, PersonRef};

/// A fixed read-only tree: `Downloads/` and `scan.pdf`. `with_write_bug` makes `write` succeed (a mutant).
#[derive(Clone, Debug, Default)]
pub struct FakeLocal {
    write_bug: bool,
    accepts: Option<&'static str>,
}

impl FakeLocal {
    /// The honest read-only provider.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A mutant that accepts one change method (`create`, `rename`, `move_to`, `copy_to`, `delete`, `restore`).
    #[must_use]
    pub fn with_accepts(mut self, method: &'static str) -> Self {
        self.accepts = Some(method);
        self
    }

    fn accepted(&self, m: &str) -> Result<Entry> {
        if self.accepts == Some(m) {
            Ok(Self::entry("local:/new", "neu", Kind::File, &sample().anna))
        } else {
            crate::provider::not_possible("storage.change")
        }
    }

    /// A mutant that saves into the foreign tree.
    #[must_use]
    pub fn with_write_bug(mut self) -> Self {
        self.write_bug = true;
        self
    }

    fn entry(id: &str, name: &str, kind: Kind, owner: &PersonRef) -> Entry {
        let file = kind == Kind::File;
        Entry {
            id: EntryId(id.to_owned()),
            name: name.to_owned(),
            kind,
            size: file.then_some(11),
            modified: None,
            media_type: file.then(|| MediaType("application/pdf".into())),
            file_ref: None,
            owner: owner.clone(),
            etag: None,
            link_target: None,
            generator: None,
        }
    }
}

#[async_trait]
impl StorageProvider for FakeLocal {
    fn capabilities(&self) -> StorageCaps {
        StorageCaps {
            range: true,
            write: self.write_bug,
            ..StorageCaps::default()
        }
    }
    async fn roots(&self) -> Result<Vec<Root>> {
        let owner = sample().anna;
        Ok(vec![Root {
            id: EntryId("local:/".into()),
            name: "Home (lokal)".into(),
            kind: RootKind::External,
            owner,
            usage: None,
        }])
    }
    async fn list(&self, at: &EntryId, _page: PageReq) -> Result<Page<Entry>> {
        if at.as_str() != "local:/" {
            return Err(Error::NotFound(at.clone()));
        }
        let o = sample().anna;
        let items = vec![
            Self::entry("local:/Downloads", "Downloads", Kind::Folder, &o),
            Self::entry("local:/scan.pdf", "scan.pdf", Kind::File, &o),
        ];
        Ok(Page { items, next: None })
    }
    async fn stat(&self, id: &EntryId) -> Result<Entry> {
        let o = sample().anna;
        match id.as_str() {
            "local:/" => Ok(Self::entry("local:/", "Home (lokal)", Kind::Folder, &o)),
            "local:/Downloads" => Ok(Self::entry(id.as_str(), "Downloads", Kind::Folder, &o)),
            "local:/scan.pdf" => Ok(Self::entry(id.as_str(), "scan.pdf", Kind::File, &o)),
            _ => Err(Error::NotFound(id.clone())),
        }
    }
    async fn open(&self, id: &EntryId, ctl: &Ctl) -> Result<Box<dyn Blob>> {
        ctl.check()?;
        let entry = self.stat(id).await?;
        if entry.kind != Kind::File {
            return Err(Error::NotFound(id.clone()));
        }
        Ok(Box::new(MemBlob::new(entry, b"local bytes".to_vec())))
    }
    async fn write(
        &self,
        at: &EntryId,
        name: &str,
        src: &mut dyn crate::ctl::ByteSource,
        _o: crate::entry::WriteOpts,
        _ctl: &Ctl,
    ) -> Result<Entry> {
        if !self.write_bug {
            return crate::provider::not_possible("storage.write");
        }
        let _ = (at, src);
        Ok(Self::entry("local:/new", name, Kind::File, &sample().anna))
    }
    async fn create(
        &self,
        _at: &EntryId,
        _n: &str,
        _k: Kind,
        _on: crate::entry::OnConflict,
    ) -> Result<Entry> {
        self.accepted("create")
    }
    async fn rename(
        &self,
        _id: &EntryId,
        _n: &str,
        _on: crate::entry::OnConflict,
    ) -> Result<Entry> {
        self.accepted("rename")
    }
    async fn move_to(
        &self,
        _id: &EntryId,
        _to: &EntryId,
        _on: crate::entry::OnConflict,
    ) -> Result<Entry> {
        self.accepted("move_to")
    }
    async fn copy_to(
        &self,
        _id: &EntryId,
        _to: &EntryId,
        _on: crate::entry::OnConflict,
        _c: &Ctl,
    ) -> Result<Entry> {
        self.accepted("copy_to")
    }
    async fn delete(&self, _id: &EntryId, _h: crate::entry::Delete) -> Result<()> {
        self.accepted("delete").map(|_| ())
    }
    async fn restore(
        &self,
        _id: &EntryId,
        _to: Option<&EntryId>,
        _on: crate::entry::OnConflict,
    ) -> Result<Entry> {
        self.accepted("restore")
    }
}
