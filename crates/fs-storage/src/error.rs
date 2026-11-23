//! The errors of `storage` as typed causes, each with its RFC 9457 problem type and status (ADR 0070).
//!
//! Pattern: Error enum (typed cause plus fields); `Display` is for developers, a person sees translations by key.

use crate::ids::EntryId;
use fs_types::Problem;
use std::fmt;

/// Why a name or path was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameProblem {
    /// Empty (also an empty path segment).
    Empty,
    /// More than 255 bytes.
    TooLong,
    /// `.` or `..`.
    Dot,
    /// Contains `/` or `\`.
    Separator,
    /// Contains a control character.
    Control,
    /// Contains a bidi override character.
    Bidi,
    /// The reserved name `.fs-files`.
    Reserved,
}

/// Errors of a storage provider.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The provider says it cannot (ADR 0045), with the capability.
    Unsupported { capability: String },
    /// The asker has no right (403). Nothing was changed.
    Forbidden { capability: String },
    /// No such entry.
    NotFound(EntryId),
    /// The name is taken in the folder (409); nothing is ever replaced.
    NameConflict { name: String },
    /// The name or path is not acceptable (422).
    InvalidName { name: String, reason: NameProblem },
    /// The request is not acceptable for another reason (422).
    Invalid(Vec<Problem>),
    /// The id is no folder.
    NotAFolder(EntryId),
    /// A link is never followed (list, open, path walk, copy, delete): the link itself is meant (ADR 0070 risk 4).
    LinkNotFollowed(EntryId),
    /// A move into itself or a loop (508).
    Cycle(EntryId),
    /// The caller cancelled (499).
    Cancelled,
    /// The time limit ran out (504).
    Timeout,
    /// The provider is not reachable (503).
    Offline,
    /// Login needed (401).
    NeedsLogin,
    /// The quota is used up (507).
    QuotaExceeded,
    /// The state changed under the call, the etag does not fit (412).
    Changed,
    /// Too large for the provider.
    TooLarge,
    /// The backend failed (disk, network).
    BackendFailed(String),
}

/// Result of a storage call.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The problem type URI under `https://freesynergy.net/problems/storage/`.
    #[must_use]
    pub fn problem_type(&self) -> &'static str {
        match self {
            Self::Unsupported { .. } => "https://freesynergy.net/problems/storage/unsupported",
            Self::Forbidden { .. } => "https://freesynergy.net/problems/storage/forbidden",
            Self::NotFound(_) => "https://freesynergy.net/problems/storage/not-found",
            Self::NameConflict { .. } => "https://freesynergy.net/problems/storage/name-conflict",
            Self::InvalidName { .. } | Self::Invalid(_) => {
                "https://freesynergy.net/problems/storage/invalid-name"
            }
            Self::NotAFolder(_) => "https://freesynergy.net/problems/storage/not-a-folder",
            Self::LinkNotFollowed(_) => {
                "https://freesynergy.net/problems/storage/link-not-followed"
            }
            Self::Cycle(_) => "https://freesynergy.net/problems/storage/cycle",
            Self::Cancelled => "https://freesynergy.net/problems/storage/cancelled",
            Self::Timeout => "https://freesynergy.net/problems/storage/timeout",
            Self::Offline => "https://freesynergy.net/problems/storage/offline",
            Self::NeedsLogin => "https://freesynergy.net/problems/storage/needs-login",
            Self::QuotaExceeded => "https://freesynergy.net/problems/storage/quota-exceeded",
            Self::Changed => "https://freesynergy.net/problems/storage/changed",
            Self::TooLarge => "https://freesynergy.net/problems/storage/too-large",
            Self::BackendFailed(_) => "https://freesynergy.net/problems/storage/backend-failed",
        }
    }

    /// The HTTP status of the problem.
    #[must_use]
    pub fn status(&self) -> u16 {
        match self {
            Self::Unsupported { .. } => 501,
            Self::Forbidden { .. } => 403,
            Self::NotFound(_) => 404,
            Self::NameConflict { .. } => 409,
            Self::InvalidName { .. }
            | Self::Invalid(_)
            | Self::NotAFolder(_)
            | Self::LinkNotFollowed(_) => 422,
            Self::Cycle(_) => 508,
            Self::Cancelled => 499,
            Self::Timeout => 504,
            Self::Offline => 503,
            Self::NeedsLogin => 401,
            Self::QuotaExceeded => 507,
            Self::Changed => 412,
            Self::TooLarge => 413,
            Self::BackendFailed(_) => 500,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.problem_type(), self.status())
    }
}
impl std::error::Error for Error {}
