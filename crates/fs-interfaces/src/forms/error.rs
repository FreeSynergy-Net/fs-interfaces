//! Why a catalog line was refused. Pattern: Value Object (an error is data with the line number).

use std::fmt;

/// Why a line was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Not of the form `name@H.N.P | display | status | description` or `> channel | art | required | count`.
    Shape,
    /// The form name is not lower-case words joined by `-`.
    Name,
    /// The version is not `H.N.P` (`SemVer`, three parts).
    Version,
    /// The status is neither `stable` nor `planned`.
    Status,
    /// The same `name@H.N.P` twice.
    DuplicateForm,
    /// A channel line before any form line.
    ChannelWithoutForm,
    /// The channel name is not lower-case words joined by `-`.
    ChannelName,
    /// The same channel twice in one form.
    DuplicateChannel,
    /// The art is not one of the seven words (or the set is empty or has a blank element).
    Art,
    /// The same art twice in one set.
    ArtDuplicate,
    /// The required mark is neither `!` nor `-`.
    Required,
    /// The count is not `1`, `a..b` or `a..n` (`?` is refused too).
    Count,
    /// A required channel whose count allows none (`0..`).
    RequiredButOptionalCount,
    /// A form without any channel.
    NoChannel,
    /// A form without a required channel (a module could fill nothing).
    NoRequiredChannel,
    /// A 5th field (`writes`) on a channel whose art is not `command`.
    WritesOnNonCommand,
    /// An entry of `writes` is not a channel name (lower-case words joined by `-`, optional trailing `!`).
    WritesName,
    /// The same channel twice in one `writes`.
    WritesDuplicate,
    /// `writes` names a channel the form does not have.
    WritesUnknownChannel,
    /// `writes` names a channel of art `command` (a command writes data channels only).
    WritesCommandChannel,
    /// `writes` names a channel whose count allows more than one field (`facts 0..4`): the gesture would have to carry the field path.
    WritesManyChannel,
    /// A relation line (`= ...`) before any form line.
    RelationWithoutForm,
    /// The kind word after `=` is not one of the known kinds (`interval`, `parent`).
    RelationKind,
    /// A known kind with the wrong number of channel names.
    RelationShape,
    /// A relation names a channel the form does not have.
    RelationUnknownChannel,
    /// A relation names a channel of the wrong art (or one that takes more than one value).
    RelationArt,
    /// The same kind twice in one form, or the same channel twice in one relation.
    RelationDuplicate,
}

/// A refused line (number counted from 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// The line number.
    pub line: usize,
    /// Why.
    pub kind: ErrorKind,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "forms line {}: {:?}", self.line, self.kind)
    }
}

impl std::error::Error for ParseError {}
