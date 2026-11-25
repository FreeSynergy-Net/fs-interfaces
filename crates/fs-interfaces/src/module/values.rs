//! Plain values of the module contract (mirror `schema/formsource.capnp`; the wire conversion comes with the stage 3 landing).
//!
//! Pattern: Value Object - data without behaviour, equal values are equal. Deviation: none.

use fs_types::Utc;

/// Where a change came from (`introduce` gave program and instance, never a right; `person` is from the connection).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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

/// Pattern: Idempotent Receiver - the same non-empty key acts once; both empty = no run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StepRef {
    /// The run (opaque text).
    pub run: String,
    /// The step in the run.
    pub step: String,
}

/// Pattern: Memento (reference) - the module holds the data, the caller only the reference.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub kind: String,
    /// When.
    pub at: Utc,
    /// At `Undo`: the change taken back.
    pub undoes: Option<String>,
}

/// One field: dotted field name of the module and the value as text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Value {
    /// The field name.
    pub path: String,
    /// The value as text.
    pub text: String,
}

/// A hidden field and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HiddenField {
    /// The field name.
    pub path: String,
    /// Why.
    pub reason: super::HiddenReason,
}

/// One record as shown to the asking person.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
pub struct Blob {
    /// The media type.
    pub media_type: String,
    /// The bytes.
    pub bytes: Vec<u8>,
}

/// Pattern: Command - one change asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
pub struct FieldClash {
    /// The field.
    pub path: String,
    /// What the record has now.
    pub current: String,
    /// What the caller wanted.
    pub mine: String,
}

/// The conflict, typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    /// The change that came in between.
    pub by: ChangeRef,
    /// The fields in conflict.
    pub fields: Vec<FieldClash>,
}

/// Pattern: Result Object - a refusal as a value: `form-source-<kind>` with a status, never a foreign sentence.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
pub struct ChangePage {
    /// The changes.
    pub changes: Vec<ChangeRef>,
    /// The cursor of the next page; empty = last.
    pub next: String,
}
