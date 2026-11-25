//! Change values of the module contract: `ChangeRef`, `ChangeKind`, `Conflict` and the `changes` query and page.
//!
//! Pattern: Value Object - data without behaviour, equal values are equal. Deviation: `#[non_exhaustive]` structs, so build them with the constructors.

use fs_types::Utc;

use super::Origin;

/// Pattern: Value Object (enum) - the AS2 kind of a change; new kinds may come, so match with a wildcard arm.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChangeKind {
    /// A record was made.
    #[default]
    Create,
    /// A record was changed.
    Update,
    /// A record was deleted.
    Delete,
    /// A change was taken back.
    Undo,
}

impl ChangeKind {
    /// The AS2 type name (`Create`, `Update`, `Delete`, `Undo`).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Create => "Create",
            Self::Update => "Update",
            Self::Delete => "Delete",
            Self::Undo => "Undo",
        }
    }
}

/// Pattern: Memento (reference) - the module holds the data, the caller only the reference.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ChangeRef {
    /// Head activity of the unit (UUIDv7 as text) = the id of the change.
    pub activity: String,
    /// The record id.
    pub object: String,
    /// The revision after the change (decimal text).
    pub revision: String,
    /// The revision before; empty at create.
    pub before: String,
    /// Where it came from.
    pub origin: Origin,
    /// AS2: `Create`, `Update`, `Delete` or `Undo`.
    pub kind: ChangeKind,
    /// When.
    pub at: Utc,
    /// At `Undo`: the change taken back.
    pub undoes: Option<String>,
    /// The fixed ids of the scopes (boards, folders) the change lies in; empty = in no scope. Corresponds to AS2 `context`.
    /// Appended (Stufe 4 V1); not on the wire yet (a card). Compared by `==` like every other field.
    pub scopes: Vec<String>,
}

impl ChangeRef {
    /// A change; `undoes` is `None`, `before` is empty at create.
    #[must_use]
    pub fn new(
        activity: &str,
        object: &str,
        revision: &str,
        before: &str,
        origin: Origin,
        kind: ChangeKind,
        at: Utc,
    ) -> Self {
        Self {
            activity: activity.into(),
            object: object.into(),
            revision: revision.into(),
            before: before.into(),
            origin,
            kind,
            at,
            undoes: None,
            scopes: Vec::new(),
        }
    }

    /// At `Undo`: the change (its activity) that is taken back.
    #[must_use]
    pub fn undoing(mut self, change: &str) -> Self {
        self.undoes = Some(change.into());
        self
    }

    /// The fixed ids of the scopes the change lies in (see [`ChangeRef::scopes`]); [`Follow::in_scope`](super::Follow::in_scope) asks them.
    #[must_use]
    pub fn in_scopes<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.scopes = ids.into_iter().map(Into::into).collect();
        self
    }
}

/// One field of a conflict: what the record has now and what the caller wanted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FieldClash {
    /// The field.
    pub path: String,
    /// What the record has now.
    pub current: String,
    /// What the caller wanted.
    pub mine: String,
    /// The record the field belongs to; empty = unknown, then the wrapper blanks `current` (it cannot tell whether the field is hidden there).
    /// Appended (Stufe 4 V1).
    pub record: String,
}

impl FieldClash {
    /// The field `path`: `current` is in the record, `mine` was wanted.
    #[must_use]
    pub fn new(path: &str, current: &str, mine: &str) -> Self {
        Self {
            path: path.into(),
            current: current.into(),
            mine: mine.into(),
            record: String::new(),
        }
    }

    /// The record the field belongs to (see [`FieldClash::record`]).
    #[must_use]
    pub fn on(mut self, record: &str) -> Self {
        self.record = record.into();
        self
    }
}

/// The conflict, typed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Conflict {
    /// The change that came in between.
    pub by: ChangeRef,
    /// The fields in conflict.
    pub fields: Vec<FieldClash>,
}

impl Conflict {
    /// The change `by` that came in between and the fields in conflict.
    #[must_use]
    pub fn new(by: ChangeRef, fields: Vec<FieldClash>) -> Self {
        Self { by, fields }
    }
}

/// What `changes` asks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ChangesQuery {
    /// Only this run; empty = the changes of the asking program, newest first.
    pub run: String,
    /// The cursor; empty at the start.
    pub cursor: String,
    /// At most this many; 0 = the default.
    pub limit: u32,
    /// Only the changes of this scope (a board, a folder; a free token the module understands, passed verbatim); empty = all.
    /// Appended (S4-B1); the wrapper asks [`ModuleCore::follows`](super::ModuleCore::follows) with it.
    pub scope: String,
    /// Only the changes of this form (`id@version`); empty = all. Appended (S4-B1).
    pub form: String,
}

/// One page of changes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ChangePage {
    /// The changes.
    pub changes: Vec<ChangeRef>,
    /// The cursor of the next page; empty = last.
    pub next: String,
}
