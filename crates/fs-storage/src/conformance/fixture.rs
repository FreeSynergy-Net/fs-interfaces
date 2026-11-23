//! The fixture (one provider per person), the report, and small helpers the rules share.
//!
//! Pattern: Test fixture (a fresh set of providers per rule) and Collecting parameter (`Report`).

use crate::ctl::Ctl;
use crate::entry::{Entry, Kind, PageReq, RootKind};
use crate::error::{Error, Result};
use crate::ids::EntryId;
use crate::provider::StorageProvider;
use sha2::{Digest, Sha256};

/// A rule's verdict: `Ok` or the reason it failed.
pub type Check = std::result::Result<(), String>;

/// The same world seen by five actors (the group owner, a member, a stranger, an admin, a bot) plus a local tree.
pub struct Fixture {
    /// Anna: owner of "My files" and of the group.
    pub anna: Box<dyn StorageProvider>,
    /// Ben: member of the group, reads only.
    pub ben: Box<dyn StorageProvider>,
    /// Fremd: nobody, no right anywhere.
    pub fremd: Box<dyn StorageProvider>,
    /// Admin: sees the structure of group folders and only the `usage` of private ones.
    pub admin: Box<dyn StorageProvider>,
    /// Bot: an AI actor, no folder, no right.
    pub bot: Box<dyn StorageProvider>,
    /// A foreign tree (local disk): import source, never a place to save to.
    pub local: Box<dyn StorageProvider>,
    /// The bot's person reference (it must never appear as an owner).
    pub bot_ref: fs_types::PersonRef,
}

/// Which rules held and which did not.
#[derive(Debug, Default)]
pub struct Report {
    /// Rules that held.
    pub passed: Vec<&'static str>,
    /// Rules that did not, with the reason.
    pub failed: Vec<(&'static str, String)>,
}

impl Report {
    pub(super) fn record(&mut self, id: &'static str, c: Check) {
        match c {
            Ok(()) => self.passed.push(id),
            Err(why) => self.failed.push((id, why)),
        }
    }
    /// Whether the rule `id` failed.
    #[must_use]
    pub fn failed_rule(&self, id: &str) -> bool {
        self.failed.iter().any(|(r, _)| *r == id)
    }
}

pub(super) fn s<T>(r: Result<T>) -> std::result::Result<T, String> {
    r.map_err(|e| e.to_string())
}

pub(super) fn want(cond: bool, msg: &str) -> Check {
    if cond { Ok(()) } else { Err(msg.to_owned()) }
}

/// `r` must be an error accepted by `f`.
pub(super) fn refused<T>(r: Result<T>, what: &str, f: fn(&Error) -> bool) -> Check {
    match r {
        Err(e) if f(&e) => Ok(()),
        Err(e) => Err(format!("{what}: wrong error {e}")),
        Ok(_) => Err(format!("{what}: was allowed")),
    }
}

pub(super) fn forbidden(e: &Error) -> bool {
    matches!(e, Error::Forbidden { .. })
}
pub(super) fn invalid_name(e: &Error) -> bool {
    matches!(e, Error::InvalidName { .. })
}
pub(super) fn conflict(e: &Error) -> bool {
    matches!(e, Error::NameConflict { .. })
}
pub(super) fn link(e: &Error) -> bool {
    matches!(e, Error::LinkNotFollowed(_))
}

/// All entries of one folder (every page).
pub(super) async fn all(
    p: &dyn StorageProvider,
    at: &EntryId,
) -> std::result::Result<Vec<Entry>, String> {
    let mut req = PageReq::first(2);
    let mut out = Vec::new();
    loop {
        let page = s(p.list(at, req.clone()).await)?;
        out.extend(page.items);
        match page.next {
            Some(c) => req.cursor = Some(c),
            None => return Ok(out),
        }
    }
}

/// Every entry below every root, by display path, links not followed.
pub(super) async fn walk(
    p: &dyn StorageProvider,
) -> std::result::Result<Vec<(String, Entry)>, String> {
    let mut out = Vec::new();
    for root in s(p.roots().await)? {
        let mut stack = vec![(root.name.clone(), root.id.clone())];
        while let Some((path, id)) = stack.pop() {
            for e in all(p, &id).await? {
                let sub = format!("{path}/{}", e.name);
                if e.kind == Kind::Folder {
                    stack.push((sub.clone(), e.id.clone()));
                }
                out.push((sub, e));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

pub(super) async fn root_of(
    p: &dyn StorageProvider,
    kind: RootKind,
) -> std::result::Result<EntryId, String> {
    let roots = s(p.roots().await)?;
    roots
        .into_iter()
        .find(|r| r.kind == kind)
        .map(|r| r.id)
        .ok_or_else(|| format!("no root of kind {kind:?}"))
}

/// The entry at a display path below the root of `kind`.
pub(super) async fn at(
    p: &dyn StorageProvider,
    kind: RootKind,
    path: &str,
) -> std::result::Result<Entry, String> {
    let root = root_of(p, kind).await?;
    s(p.resolve_path(&root, path).await)
}

pub(super) async fn read_all(
    p: &dyn StorageProvider,
    e: &Entry,
) -> std::result::Result<Vec<u8>, String> {
    let blob = s(p.open(&e.id, &Ctl::none()).await)?;
    s(blob.read(0, u64::MAX).await)
}

pub(super) fn sha(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
