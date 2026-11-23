//! The vocabulary of the rights matrix (RECHTE-MATRIX.md section 1): actors, target kinds, expected results, methods.
//!
//! Pattern: Value objects and a parser for the cell strings of the table.

use super::methods::Method;
use crate::error::Error;

/// The six actors, in the order of the table columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Actor {
    /// Owner (Anna).
    E,
    /// Member (Ben).
    M,
    /// Stranger.
    F,
    /// Manager (admin level (a)/(b), no server access).
    V,
    /// Operator (server access).
    B,
    /// Bot.
    K,
}

/// All actors in column order.
pub const ACTORS: [Actor; 6] = [Actor::E, Actor::M, Actor::F, Actor::V, Actor::B, Actor::K];

/// The target kinds T1..T13 (section 1.4) plus `T8B` (below a trashed folder, 3.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    /// Own entry.
    T1,
    /// Foreign private entry.
    T2,
    /// Own root.
    T3,
    /// Foreign root.
    T4,
    /// Group root.
    T5,
    /// Group entry.
    T6,
    /// Trash root.
    T7,
    /// Own trashed entry.
    T8,
    /// Foreign private trashed entry.
    T9,
    /// Group trashed entry.
    T10,
    /// Own link.
    T11,
    /// Foreign link.
    T12,
    /// Read-only tree.
    T13,
    /// A folder below a trashed folder.
    T8B,
}

/// The operations of the table: one call each, with source and target axes apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    /// `stat` of a file.
    Stat,
    /// `list` of a folder.
    List,
    /// `open` of a file.
    Open,
    /// `write` below a folder.
    Write,
    /// `write` with a cancelled `ctl` (rights before the body).
    WriteCancelled,
    /// `create` of a folder.
    Create,
    /// `create` with a kind other than folder.
    CreateFile,
    /// `rename` of the target.
    Rename,
    /// `move_to`, the target is the source (to an own folder).
    MoveSrc,
    /// `move_to`, the target is the destination (source: own file).
    MoveDst,
    /// `move_to` of an own folder into its own child.
    MoveCycle,
    /// `copy_to`, the target is the source.
    CopySrc,
    /// `copy_to`, the target is the destination.
    CopyDst,
    /// `copy_to` of an own folder into its own child.
    CopyCycle,
    /// `delete` to the trash.
    Trash,
    /// `delete` for good.
    Purge,
    /// `restore` of the target.
    RestoreSrc,
    /// `restore` of an own trashed file to the target.
    RestoreDst,
}

impl Op {
    /// The id used in rule names.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Stat => "stat",
            Self::List => "list",
            Self::Open => "open",
            Self::Write => "write",
            Self::WriteCancelled => "write_cancelled",
            Self::Create => "create",
            Self::CreateFile => "create_file",
            Self::Rename => "rename",
            Self::MoveSrc => "move_src",
            Self::MoveDst => "move_dst",
            Self::MoveCycle => "move_cycle",
            Self::CopySrc => "copy_src",
            Self::CopyDst => "copy_dst",
            Self::CopyCycle => "copy_cycle",
            Self::Trash => "trash",
            Self::Purge => "purge",
            Self::RestoreSrc => "restore_src",
            Self::RestoreDst => "restore_dst",
        }
    }

    /// The trait method this operation exercises.
    #[must_use]
    pub fn method(self) -> Method {
        match self {
            Self::Stat => Method::Stat,
            Self::List => Method::List,
            Self::Open => Method::Open,
            Self::Write | Self::WriteCancelled => Method::Write,
            Self::Create | Self::CreateFile => Method::Create,
            Self::Rename => Method::Rename,
            Self::MoveSrc | Self::MoveDst | Self::MoveCycle => Method::MoveTo,
            Self::CopySrc | Self::CopyDst | Self::CopyCycle => Method::CopyTo,
            Self::Trash | Self::Purge => Method::Delete,
            Self::RestoreSrc | Self::RestoreDst => Method::Restore,
        }
    }
}

/// An expected result (section 1.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expect {
    /// `Ok`.
    Ok,
    /// `Forbidden`.
    Fb,
    /// `NotFound`.
    Nf,
    /// `Unsupported`.
    Us,
    /// `LinkNotFollowed`.
    Lnf,
    /// `Invalid` or `NotAFolder`.
    Inv,
    /// `InvalidName`.
    In,
    /// `NameConflict`.
    Cf,
    /// `Cycle`.
    Cy,
    /// `Cancelled`.
    Ca,
    /// Any error except `Unsupported`.
    Ref,
    /// Not applicable: no call.
    Na,
}

impl Expect {
    /// Parse one cell token; a trailing `?` marks a provisional cell (decided by the strategy, 23.11.).
    ///
    /// # Panics
    /// On an unknown token (the table is code, a typo must stop the tests).
    #[must_use]
    pub fn parse(tok: &str) -> (Self, bool) {
        let (t, prov) = tok.strip_suffix('?').map_or((tok, false), |t| (t, true));
        let e = match t {
            "OK" => Self::Ok,
            "FB" => Self::Fb,
            "NF" => Self::Nf,
            "US" => Self::Us,
            "LNF" => Self::Lnf,
            "INV" => Self::Inv,
            "IN" => Self::In,
            "CF" => Self::Cf,
            "CY" => Self::Cy,
            "CA" => Self::Ca,
            "REF" => Self::Ref,
            "NA" => Self::Na,
            other => panic!("unknown matrix code {other}"),
        };
        (e, prov)
    }

    /// Whether `r` (the outcome of the call) is what the cell expects.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub fn holds<T>(self, r: &Result<T, Error>) -> bool {
        match (self, r) {
            (Self::Ok, Ok(_)) | (Self::Na, _) => true,
            (Self::Ref, Err(e)) => !matches!(e, Error::Unsupported { .. }),
            (Self::Fb, Err(Error::Forbidden { .. }))
            | (Self::Nf, Err(Error::NotFound(_)))
            | (Self::Us, Err(Error::Unsupported { .. }))
            | (Self::Lnf, Err(Error::LinkNotFollowed(_)))
            | (Self::Inv, Err(Error::Invalid(_) | Error::NotAFolder(_)))
            | (Self::In, Err(Error::InvalidName { .. }))
            | (Self::Cf, Err(Error::NameConflict { .. }))
            | (Self::Cy, Err(Error::Cycle(_)))
            | (Self::Ca, Err(Error::Cancelled)) => true,
            _ => false,
        }
    }
}

/// One row of the table: an operation on a target kind, six cells (E M F V B K), and the mutant that must break it.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    /// The operation.
    pub op: Op,
    /// The target kind.
    pub target: Target,
    /// Six cell tokens separated by blanks.
    pub cells: &'static str,
    /// The `Fault` names that make this row fail, or `-: <reason>` for a row without a mutant.
    pub mutant: &'static str,
}

impl Row {
    /// The six parsed cells.
    ///
    /// # Panics
    /// When the row has not exactly six cells.
    #[must_use]
    pub fn parsed(&self) -> Vec<(Expect, bool)> {
        let v: Vec<_> = self.cells.split_whitespace().map(Expect::parse).collect();
        assert_eq!(v.len(), 6, "row {self:?} needs 6 cells");
        v
    }

    /// The rule name in a report.
    #[must_use]
    pub fn rule(&self) -> String {
        format!("matrix:{}:{:?}", self.op.name(), self.target)
    }
}
