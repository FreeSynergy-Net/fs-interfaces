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
    /// A manager (admin without server access) reads the content of every folder.
    AdminReadsAll,
    /// The operator is treated like any other asker (gets 403 or only `usage`).
    OperatorBlocked,
    /// `resolve_path` interprets `..` and empty segments (path traversal).
    PathTraversal,
    /// The operator may also change foreign entries (without an intervention).
    OperatorChanges,
    /// A manager may also change content in a group folder (without an intervention).
    ManagerChanges,
    /// `move_to` does not check the right on the TARGET folder.
    MoveIgnoresTarget,
    /// `copy_to` does not check the right on the target folder.
    CopyIgnoresTarget,
    /// `restore` to another folder does not check the right on it.
    RestoreIgnoresTarget,
    /// A manager may write and create in the root folder of a group.
    ManagerGroupRoot,
    /// The operator may write and create in a root folder ("My files" of others, group roots).
    OperatorRoot,
    /// The operator may change links of others.
    OperatorLink,
    /// The cap of one read is not kept.
    NoReadCap,
    /// A read answers nothing before the end.
    ZeroRead,
    /// A read ignores the length asked and answers more.
    ReadsTooMuch,
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
    /// Managers: admin level (a)/(b) without server access (structure of groups, `usage` of private folders).
    pub admins: Vec<PersonRef>,
    /// Operators: whoever has server (root) access. In the fake only a list the test sets; assigning it is no business of this
    /// contract (the group contract, scope `Server`, local command only).
    pub operators: Vec<PersonRef>,
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
    pub(super) read_limit: u64,
}

impl FakeStorage {
    /// The sample tree, seen by `asker`.
    #[must_use]
    pub fn new(asker: &PersonRef) -> Self {
        Self {
            world: Arc::new(Mutex::new(super::sample::build())),
            asker: asker.clone(),
            fault: None,
            read_limit: crate::ctl::MAX_READ,
        }
    }

    /// The same world seen by another person.
    #[must_use]
    pub fn as_person(&self, who: &PersonRef) -> Self {
        Self {
            world: Arc::clone(&self.world),
            asker: who.clone(),
            fault: self.fault,
            read_limit: self.read_limit,
        }
    }

    /// A provider that answers at most `limit` bytes per read (a short read, as the contract allows).
    #[must_use]
    pub fn with_read_limit(mut self, limit: u64) -> Self {
        self.read_limit = limit;
        self
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
