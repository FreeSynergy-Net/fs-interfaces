//! The operator's read log (decision 16.2 / F3): who gets a `Viewed` entry and who does not.
//!
//! Pattern: Rule (information only: the log records access through the program and protects nothing).

use super::fixture::{Check, Fixture, read_all, s, walk, want};
use crate::ctl::Ctl;
use crate::entry::{OnConflict, RootKind};

/// Decision 16.2 / F3: when the operator opens or copies foreign content, the owner gets an entry (information only: it
/// records access through the program and protects nothing). Nobody else sees it; the copy belongs to the operator.
pub(super) async fn operator_log(fx: &Fixture) -> Check {
    let f = super::fixture::at(&*fx.anna, RootKind::Mine, "Dokumente/Notizen.md").await?;
    let t0 = s(fx.anna.changes(None).await)?.token;
    read_all(&*fx.operator, &f).await?;
    let opened = |set: &crate::entry::ChangeSet| {
        set.changes
            .iter()
            .filter(|c| c.id == f.id && c.kind == crate::entry::ChangeKind::Viewed)
            .count()
    };
    let one = s(fx.anna.changes(Some(&t0)).await)?;
    want(
        opened(&one) == 1,
        "the owner gets no log entry when the operator opens a file",
    )?;
    want(
        s(fx.fremd.changes(Some(&t0)).await)?.changes.is_empty(),
        "a stranger sees the operator's log entry",
    )?;
    let roots = s(fx.operator.roots().await)?;
    let trash = roots
        .iter()
        .find(|r| r.kind == RootKind::Trash)
        .ok_or("no trash root")?;
    let own = roots
        .iter()
        .find(|r| r.kind == RootKind::Mine && r.owner.id == trash.owner.id)
        .ok_or("no own root")?;
    let copy = s(fx
        .operator
        .copy_to(&f.id, &own.id, OnConflict::KeepBoth, &Ctl::none())
        .await)?;
    want(
        copy.owner == trash.owner,
        "a copy made by the operator must belong to the operator",
    )?;
    let t1 = s(fx.anna.changes(Some(&t0)).await)?.token;
    want(
        opened(&s(fx.anna.changes(Some(&t0)).await)?) == 2,
        "the owner gets no log entry when the operator copies a file",
    )?;
    // Counter-check: a member who reads a group file is no operator and leaves no entry (never unfair to the reader).
    let all = walk(&*fx.anna).await?;
    let (_, gfile) = all
        .iter()
        .find(|(p, _)| p == "Helfa Ortsgruppe (Beispieldaten)/Regeln/Konfliktstufen.md")
        .ok_or("no group file")?;
    read_all(&*fx.ben, gfile).await?;
    want(
        s(fx.anna.changes(Some(&t1)).await)?.changes.is_empty(),
        "a member who reads a group file leaves a view entry (only the operator's reads are logged)",
    )
}
