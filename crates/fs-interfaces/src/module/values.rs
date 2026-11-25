//! Plain values of the module contract (mirror `schema/formsource.capnp`; the wire conversion comes with the stage 3 landing).
//!
//! Pattern: Value Object - data without behaviour, equal values are equal. Deviation: the structs are `#[non_exhaustive]` (fields
//! may be appended later), so code outside this crate builds them with `Default` plus assignment or a constructor, not a literal.

use fs_types::Utc;

/// Where a change came from (`introduce` gave program and instance, never a right; `person` is from the connection).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Origin {
    /// IRI of the person.
    pub person: String,
    /// `AppStream` id of the program; empty allowed.
    pub program: String,
    /// Instance of the program; empty allowed.
    pub instance: String,
    /// Device key; empty allowed.
    pub device: String,
}

impl Origin {
    /// An origin of `person` with program, instance and device (empty allowed).
    #[must_use]
    pub fn new(person: &str, program: &str, instance: &str, device: &str) -> Self {
        Self {
            person: person.into(),
            program: program.into(),
            instance: instance.into(),
            device: device.into(),
        }
    }
}

/// Pattern: Idempotent Receiver - the same non-empty key acts once; both empty = no run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct StepRef {
    /// The run (opaque text).
    pub run: String,
    /// The step in the run.
    pub step: String,
}

impl StepRef {
    /// The step `step` of the run `run`.
    #[must_use]
    pub fn new(run: &str, step: &str) -> Self {
        Self {
            run: run.into(),
            step: step.into(),
        }
    }
}

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
        }
    }

    /// At `Undo`: the change (its activity) that is taken back.
    #[must_use]
    pub fn undoing(mut self, change: &str) -> Self {
        self.undoes = Some(change.into());
        self
    }
}

/// One field: dotted field name of the module and the value as text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Value {
    /// The field name.
    pub path: String,
    /// The value as text.
    pub text: String,
}

impl Value {
    /// The field `path` with the value `text`.
    #[must_use]
    pub fn new(path: &str, text: &str) -> Self {
        Self {
            path: path.into(),
            text: text.into(),
        }
    }
}

/// A hidden field and why.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct HiddenField {
    /// The field name.
    pub path: String,
    /// Why.
    pub reason: super::HiddenReason,
}

impl HiddenField {
    /// The field `path`, hidden for `reason`.
    #[must_use]
    pub fn new(path: &str, reason: super::HiddenReason) -> Self {
        Self {
            path: path.into(),
            reason,
        }
    }
}

/// One record as shown to the asking person.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct RecordView {
    /// The record id.
    pub id: String,
    /// Commands the person may run on it (filled by the wrapper from the rules).
    pub may: Vec<String>,
    /// Hidden fields.
    pub hidden: Vec<HiddenField>,
    /// The values.
    pub values: Vec<Value>,
}

/// What `page` asks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PageQuery {
    /// The form key `name@x.y.z`.
    pub form: String,
    /// The scope.
    pub scope: String,
    /// The cursor of the last page; empty at the start.
    pub cursor: String,
    /// At most this many; 0 = the default of the service.
    pub limit: u32,
}

/// One page of records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Page {
    /// The records.
    pub records: Vec<RecordView>,
    /// The exact total, if known.
    pub total: Option<u64>,
    /// An estimated total, if no exact one.
    pub estimate: Option<u64>,
    /// Commands the person may run on the set.
    pub may: Vec<String>,
    /// The cursor of the next page; empty = last page.
    pub next: String,
}

/// The bytes of a binary field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Blob {
    /// The media type.
    pub media_type: String,
    /// The bytes.
    pub bytes: Vec<u8>,
}

/// Pattern: Command - one change asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Invocation {
    /// The key of the contract (`card.move`).
    pub command: String,
    /// The record it is about, if any.
    pub record: Option<String>,
    /// Arguments, by field name of the module.
    pub args: Vec<Value>,
    /// The revision the caller saw (compare-and-set); `None` = no check.
    pub expect: Option<String>,
    /// Idempotency key; `None` = no run.
    pub step: Option<StepRef>,
}

/// What a write made or changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Applied {
    /// The record it points at; empty = none.
    pub record: String,
    /// A short code for people.
    pub text: String,
    /// The change made, if one was recorded.
    pub change: Option<ChangeRef>,
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
}

impl FieldClash {
    /// The field `path`: `current` is in the record, `mine` was wanted.
    #[must_use]
    pub fn new(path: &str, current: &str, mine: &str) -> Self {
        Self {
            path: path.into(),
            current: current.into(),
            mine: mine.into(),
        }
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

/// Pattern: Result Object - a refusal as a value: `form-source-<kind>` with a status, never a foreign sentence.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Refusal {
    /// The kind (`forbidden`, `conflict`, ...).
    pub kind: String,
    /// The HTTP-like status (403, 409, ...).
    pub status: u16,
    /// A code or short text for people.
    pub detail: String,
    /// At `conflict`: the details.
    pub conflict: Option<Conflict>,
}

impl Refusal {
    /// `form-source-conflict` 409 with the typed conflict.
    #[must_use]
    pub fn conflict(detail: &str, conflict: Conflict) -> Self {
        Self {
            kind: "conflict".into(),
            status: 409,
            detail: detail.into(),
            conflict: Some(conflict),
        }
    }

    /// `form-source-forbidden` 403 for a rules `Deny`.
    #[must_use]
    pub fn forbidden(reason: &str) -> Self {
        Self {
            kind: "forbidden".into(),
            status: 403,
            detail: reason.into(),
            conflict: None,
        }
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
