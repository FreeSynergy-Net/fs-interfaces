//! Where a process takes the local owner from (the person whose device and store this is).
//!
//! Pattern: Strategy - the source of the owner is exchangeable (single place: the module `person`; test: a fake; later
//! other sources) while the flow stays the same. Deviation: synchronous and without fallback; there is no
//! "anonymous", no empty owner and no placeholder, without a "me" it fails loudly with a reason.
//!
//! Nothing here depends on a runtime, on fs-rpc or on the people contract; the value is a [`PersonRef`].

use std::fmt;

use fs_types::PersonRef;

#[cfg(feature = "fake")]
pub mod fake;

/// Pattern: Strategy - where a process reads the local owner from, exchangeable (see the module doc).
///
/// Read once per process, before the store is opened. A real source may block on a thread of its own.
pub trait LocalOwner: Send + Sync {
    /// The stable id of the local owner: `PersonRef { id: urn:uuid:<UUIDv7>, art: Person, abzug: None }`
    /// (ADR 0085:59, 0027).
    ///
    /// # Errors
    /// [`OwnerProblem`]: there is no owner, the source is not reachable, or its answer is not a valid owner.
    fn owner(&self) -> Result<PersonRef, OwnerProblem>;
}

/// Pattern: Value Object - why there is no owner. Every kind has a code for stderr and messages.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OwnerProblem {
    /// `owner-no-me`: the module `person` runs, but "me" is not created.
    NoMe {
        /// What to do about it, in words for a person.
        hint: String,
    },
    /// `owner-unreachable`: the module `person` is missing or does not answer.
    Unreachable {
        /// Why it could not be reached.
        reason: String,
    },
    /// `owner-invalid`: the answer is not a `urn:uuid` v7, or its kind is not `Person`.
    Invalid {
        /// What is wrong with the answer.
        reason: String,
    },
}

impl OwnerProblem {
    /// Code of [`OwnerProblem::NoMe`].
    pub const NO_ME: &'static str = "owner-no-me";
    /// Code of [`OwnerProblem::Unreachable`].
    pub const UNREACHABLE: &'static str = "owner-unreachable";
    /// Code of [`OwnerProblem::Invalid`].
    pub const INVALID: &'static str = "owner-invalid";

    /// The fixed code of this kind (stable, for stderr and for tests).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::NoMe { .. } => Self::NO_ME,
            Self::Unreachable { .. } => Self::UNREACHABLE,
            Self::Invalid { .. } => Self::INVALID,
        }
    }

    /// The reason or hint, in words.
    #[must_use]
    pub fn reason(&self) -> &str {
        match self {
            Self::NoMe { hint: s }
            | Self::Unreachable { reason: s }
            | Self::Invalid { reason: s } => s,
        }
    }
}

impl fmt::Display for OwnerProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.reason())
    }
}

impl std::error::Error for OwnerProblem {}
