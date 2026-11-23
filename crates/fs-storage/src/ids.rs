//! `EntryId` and the name rules: the one place that refuses traversal and hidden control characters.
//!
//! Pattern: Value object (an opaque id) and Guard (names and paths are checked before any provider sees them).

use crate::error::{Error, NameProblem, Result};

/// Longest name in bytes (the rule of `files.capnp`).
pub const MAX_NAME_BYTES: usize = 255;
/// The name the file program keeps for its own folder; nobody may create it.
pub const RESERVED_NAME: &str = ".fs-files";

/// The id of an entry: opaque text. Paths are for display, the id is the truth (ADR 0070 point 1).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntryId(pub String);

impl EntryId {
    /// The id as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Check one name (a file or folder name, one path segment).
///
/// Refused: empty, longer than 255 bytes, `.` and `..`, `/` and `\`, control characters (C0, C1, NUL), the bidi
/// override characters that make a name look like another one, and `.fs-files`.
///
/// # Errors
/// `Error::InvalidName` with the reason.
pub fn validate_name(name: &str) -> Result<()> {
    let why = if name.is_empty() {
        Some(NameProblem::Empty)
    } else if name.len() > MAX_NAME_BYTES {
        Some(NameProblem::TooLong)
    } else if name == "." || name == ".." {
        Some(NameProblem::Dot)
    } else if name.contains(['/', '\\']) {
        Some(NameProblem::Separator)
    } else if name.chars().any(char::is_control) {
        Some(NameProblem::Control)
    } else if name.chars().any(is_bidi) {
        Some(NameProblem::Bidi)
    } else if name == RESERVED_NAME {
        Some(NameProblem::Reserved)
    } else {
        None
    };
    match why {
        None => Ok(()),
        Some(reason) => Err(Error::InvalidName {
            name: name.chars().take(40).collect(),
            reason,
        }),
    }
}

fn is_bidi(c: char) -> bool {
    matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}')
}

/// Split a display path `a/b/c` below a root into checked segments. The empty path is the root itself. No segment may
/// be empty (so no leading `/`, no `//`), `.` or `..`.
///
/// # Errors
/// `Error::InvalidName` for the first bad segment.
pub fn split_path(path: &str) -> Result<Vec<&str>> {
    if path.is_empty() {
        return Ok(Vec::new());
    }
    path.split('/')
        .map(|s| validate_name(s).map(|()| s))
        .collect()
}
