//! The state of the fake: nodes, groups, the change log, and the faults that turn it into a mutant.
//!
//! Pattern: Fake state behind one lock; `Fault` is a switch for conformance mutants (never set in normal use).

use crate::entry::{Change, Entry, Root};
use fs_types::PersonRef;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// A deliberate defect, to prove that a conformance rule really catches it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// No right is checked.
    IgnoreRights,
    /// Names are not checked (traversal, control characters).
    SkipNameCheck,
    /// A link is followed on `list`.
    FollowLinks,
    /// The sha256 in `file_ref` is wrong.
    WrongSha,
    /// A name conflict silently replaces the old entry.
    Replace,
    /// `Trash` purges.
    PurgeOnTrash,
    /// The trash shows entries the asker may not read.
    TrashLeak,
    /// A cancelled `write` is not noticed.
    NoCancel,
    /// A bot gets rights like a person.
    BotAllowed,
    /// An admin reads the content of every folder.
    AdminReadsAll,
    /// `resolve_path` interprets `..` and empty segments (path traversal).
    PathTraversal,
    /// A range read ignores its offset.
    BadRange,
    /// `scope` shows entries the asker may not read.
    ScopeLeak,
}

/// One entry with its place and bytes.
#[derive(Clone, Debug)]
pub(super) struct Node {
    pub entry: Entry,
    pub parent: Option<String>,
    /// Set while in the trash: the folder it came from.
    pub trashed_from: Option<String>,
}

/// A group with its owners and members (the real module asks the group contract).
#[derive(Clone, Debug)]
pub(super) struct Group {
    pub who: PersonRef,
    pub owners: Vec<PersonRef>,
    pub members: Vec<PersonRef>,
}

#[derive(Default)]
pub(super) struct World {
    /// Content-addressed bytes by `urn:sha256:` reference (the shape of the v5 `FileStore`, here in a plain map).
    pub blobs: std::collections::HashMap<String, Vec<u8>>,
    pub admins: Vec<PersonRef>,
    pub bots: Vec<PersonRef>,
    pub nodes: BTreeMap<String, Node>,
    pub roots: Vec<Root>,
    pub groups: Vec<Group>,
    pub seq: u32,
    pub log: Vec<(Change, PersonRef)>,
}

/// The fake provider: shared world, one asker.
#[derive(Clone)]
pub struct FakeStorage {
    pub(super) world: Arc<Mutex<World>>,
    pub(super) asker: PersonRef,
    pub(super) fault: Option<Fault>,
}

impl FakeStorage {
    /// The sample tree, seen by `asker`.
    #[must_use]
    pub fn new(asker: &PersonRef) -> Self {
        Self {
            world: Arc::new(Mutex::new(super::sample::build())),
            asker: asker.clone(),
            fault: None,
        }
    }

    /// The same world seen by another person.
    #[must_use]
    pub fn as_person(&self, who: &PersonRef) -> Self {
        Self {
            world: Arc::clone(&self.world),
            asker: who.clone(),
            fault: self.fault,
        }
    }

    /// Switch a defect on (mutant).
    #[must_use]
    pub fn with_fault(mut self, fault: Fault) -> Self {
        self.fault = Some(fault);
        self
    }
}

impl std::fmt::Debug for FakeStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FakeStorage({})",
            fs_types::BasicValue::canonical(&self.asker.id)
        )
    }
}
