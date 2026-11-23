//! The methods of the traits as a list (the meta test compares it with the trait source).
//!
//! Pattern: Value objects (an enum with an exhaustive `name`).

/// Every method of `StorageProvider` and `Blob` (the meta test compares this list with the source of the traits).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Method {
    /// `capabilities`.
    Capabilities,
    /// `state`.
    State,
    /// `roots`.
    Roots,
    /// `list`.
    List,
    /// `stat`.
    Stat,
    /// `open`.
    Open,
    /// `write`.
    Write,
    /// `create`.
    Create,
    /// `rename`.
    Rename,
    /// `move_to`.
    MoveTo,
    /// `copy_to`.
    CopyTo,
    /// `delete`.
    Delete,
    /// `restore`.
    Restore,
    /// `watch`.
    Watch,
    /// `scope`.
    Scope,
    /// `changes`.
    Changes,
    /// `resolve_path`.
    ResolvePath,
    /// `Blob::entry`.
    BlobEntry,
    /// `Blob::read`.
    BlobRead,
}

/// All methods.
pub const METHODS: [Method; 19] = [
    Method::Capabilities,
    Method::State,
    Method::Roots,
    Method::List,
    Method::Stat,
    Method::Open,
    Method::Write,
    Method::Create,
    Method::Rename,
    Method::MoveTo,
    Method::CopyTo,
    Method::Delete,
    Method::Restore,
    Method::Watch,
    Method::Scope,
    Method::Changes,
    Method::ResolvePath,
    Method::BlobEntry,
    Method::BlobRead,
];

impl Method {
    /// The name in the trait source (`fn <name>(`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Capabilities => "capabilities",
            Self::State => "state",
            Self::Roots => "roots",
            Self::List => "list",
            Self::Stat => "stat",
            Self::Open => "open",
            Self::Write => "write",
            Self::Create => "create",
            Self::Rename => "rename",
            Self::MoveTo => "move_to",
            Self::CopyTo => "copy_to",
            Self::Delete => "delete",
            Self::Restore => "restore",
            Self::Watch => "watch",
            Self::Scope => "scope",
            Self::Changes => "changes",
            Self::ResolvePath => "resolve_path",
            Self::BlobEntry => "entry",
            Self::BlobRead => "read",
        }
    }
}
