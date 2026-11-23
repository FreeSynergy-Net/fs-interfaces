//! The value types of `storage`: entries, roots, capabilities, paging, conflict and delete modes, changes.
//!
//! Pattern: Value object (plain data compared by value).

use crate::ids::EntryId;
use fs_types::{FileRef, MediaType, PageCursor, PersonRef, Utc};

/// What an entry is. A `Link` is never followed; an `App` entry is a tree of another program (read-only, export form).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// The provider does not know.
    Unknown,
    /// A file.
    File,
    /// A folder.
    Folder,
    /// A link to another entry (`link_target`).
    Link,
    /// An entry of a program tree (ADR 0070 point 5).
    App,
}

/// One entry. AS2: file = `Document`/`Image`/`Video`/`Audio` by media type, folder = `Collection`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Opaque id (the truth); the name is display.
    pub id: EntryId,
    /// The real name.
    pub name: String,
    /// What it is.
    pub kind: Kind,
    /// Bytes; `None` = unknown (folders, links, apps).
    pub size: Option<u64>,
    /// Last change.
    pub modified: Option<Utc>,
    /// Media type, where known.
    pub media_type: Option<MediaType>,
    /// The file with its sha256 (what other modules refer to); `None` for what is no file and for foreign entries
    /// that are not imported yet (an `App` entry or a file on a local disk has no checksum before the import).
    pub file_ref: Option<FileRef>,
    /// Whose it is: a person or a group (an AI never).
    pub owner: PersonRef,
    /// State marker for `Changed`.
    pub etag: Option<String>,
    /// The target of a `Link`.
    pub link_target: Option<EntryId>,
    /// What made it (AS2 `generator`), e.g. `capture`. A gallery is a `scope` over a generator, not a folder.
    pub generator: Option<String>,
}

/// The kind of a root.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RootKind {
    /// "My files" of the asker.
    Mine,
    /// The shared folder of a group.
    Group,
    /// The trash.
    Trash,
    /// A foreign tree (local disk, USB, S3, WebDAV): import source and export target, never a place to save to.
    External,
}

/// How much a root holds (rclone `About`): what a manager (admin level (a)/(b) without server access)
/// sees of a private folder. The operator (server access) reads everything and is not limited to this.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Usage {
    /// Bytes of all files.
    pub bytes: u64,
    /// Number of files.
    pub items: u64,
}

/// A place the asker may start from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Root {
    /// The id of the folder entry.
    pub id: EntryId,
    /// Display name.
    pub name: String,
    /// Kind.
    pub kind: RootKind,
    /// Whose it is.
    pub owner: PersonRef,
    /// Size and count; `None` where the provider does not know.
    pub usage: Option<Usage>,
}

/// What a provider can do (ADR 0070 `StorageCaps`); the window shows the rest locked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct StorageCaps {
    /// `create`.
    pub create: bool,
    /// `write`.
    pub write: bool,
    /// `rename`.
    pub rename: bool,
    /// `move_to`.
    pub move_to: bool,
    /// `copy_to`.
    pub copy: bool,
    /// `delete` with `Trash`.
    pub trash: bool,
    /// `restore`.
    pub restore: bool,
    /// Range reads (always true for a conforming provider).
    pub range: bool,
    /// `watch` (push).
    pub watch: bool,
    /// `changes(since)` (delta).
    pub delta_since: bool,
    /// `scope` (everything of one generator).
    pub scope: bool,
}

/// State of the provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderState {
    /// Works.
    Ready,
    /// Login needed.
    NeedsLogin,
    /// Not reachable.
    Offline,
    /// Reads only.
    ReadOnly,
}

/// Sort order of a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    /// Folders first, then by name.
    Name,
    /// Newest first.
    Modified,
    /// Largest first.
    Size,
}

/// A page request: one level only, never recursive (ADR 0070 risk 3).
#[derive(Clone, Debug)]
pub struct PageReq {
    /// Where to go on; `None` = first page.
    pub cursor: Option<PageCursor>,
    /// Entries per page, 1 to 1000.
    pub size: usize,
    /// Order.
    pub sort: Sort,
}

impl PageReq {
    /// First page of `size` entries by name.
    #[must_use]
    pub fn first(size: usize) -> Self {
        Self {
            cursor: None,
            size,
            sort: Sort::Name,
        }
    }
}

/// What to do when the name is taken. There is no "replace" (ADR 0070).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnConflict {
    /// Refuse with `NameConflict`.
    Fail,
    /// Keep both: the new one gets `name (2)`.
    KeepBoth,
}

/// How to delete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Delete {
    /// Into the trash (undoable).
    Trash,
    /// Gone for good (not undoable; the window says so before).
    Purge,
}

/// Options of `write`.
#[derive(Clone, Debug)]
pub struct WriteOpts {
    /// Name conflict mode.
    pub on_conflict: OnConflict,
    /// Media type the caller knows.
    pub media_type: Option<MediaType>,
    /// What makes the file (AS2 `generator`).
    pub generator: Option<String>,
}

/// The kind of a change; `as2` is the activity it is logged as (ADR 0020).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    /// New entry.
    Created,
    /// Renamed or written again.
    Updated,
    /// Moved.
    Moved,
    /// Into the trash.
    Trashed,
    /// Out of the trash.
    Restored,
    /// Gone for good.
    Purged,
    /// The operator opened or copied the entry through the program (decision 16.2). Information for the owner only: it
    /// records access through the program and protects nothing (whoever is root can read past it).
    OpenedByOperator,
}

impl ChangeKind {
    /// The AS2 activity type.
    #[must_use]
    pub fn as2(self) -> &'static str {
        match self {
            Self::Created => "Create",
            Self::Updated => "Update",
            Self::Moved => "Add",
            Self::Trashed | Self::Purged => "Delete",
            Self::Restored => "Undo",
            Self::OpenedByOperator => "View",
        }
    }
}

/// One change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    /// The entry.
    pub id: EntryId,
    /// What happened.
    pub kind: ChangeKind,
}

/// A place in the change log (opaque).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token(pub String);

/// Changes since a token, with the token for the next call.
#[derive(Clone, Debug)]
pub struct ChangeSet {
    /// Oldest first.
    pub changes: Vec<Change>,
    /// Ask again with this.
    pub token: Token,
}
