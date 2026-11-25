//! Plain values of the module contract (mirror `schema/formsource.capnp`; the wire conversion comes with the stage 3 landing).
//!
//! Pattern: Value Object - data without behaviour, equal values are equal. Deviation: the structs are `#[non_exhaustive]` (fields
//! may be appended later), so code outside this crate builds them with `Default` plus assignment or a constructor, not a literal.

use super::{ChangeRef, Conflict};

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

    /// `form-source-unknown-scope` 404: the module does not know the scope (see
    /// [`ModuleCore::resolve_scope`](super::ModuleCore::resolve_scope)).
    #[must_use]
    pub fn unknown_scope(detail: &str) -> Self {
        Self {
            kind: "unknown-scope".into(),
            status: 404,
            detail: detail.into(),
            conflict: None,
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
