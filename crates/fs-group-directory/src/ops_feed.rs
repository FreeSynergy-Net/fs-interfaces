//! The change stream and the log of a group: `changes`, `history`.
//!
//! Pattern: Observer feed (a numbered list; the cursor is the number last seen; what names a group the asker may
//! not see is left out).

use crate::groups_v1::shared::{ObjectId, Page, Place};
use crate::groups_v1::{Change, ChangeKind, HistoryEntry, HistoryKind, Result, Right};
use crate::paging::page;
use crate::state::State;

/// The groups a change names.
fn groups_of_change(kind: &ChangeKind) -> Vec<ObjectId> {
    match kind {
        ChangeKind::Group(g) | ChangeKind::Deleted(g) | ChangeKind::Restored(g) => vec![*g],
        ChangeKind::Membership { group }
        | ChangeKind::Invitation { group }
        | ChangeKind::Grant { group }
        | ChangeKind::Mirror { group } => vec![*group],
        ChangeKind::Nesting { child, parent } | ChangeKind::NestingRequest { child, parent } => {
            vec![*child, *parent]
        }
    }
}

/// The other group a log entry names, when it names one.
fn other_of(kind: &HistoryKind) -> Option<ObjectId> {
    match kind {
        HistoryKind::NestingLinked { other, .. }
        | HistoryKind::NestingRequested { other, .. }
        | HistoryKind::NestingAnswered { other, .. }
        | HistoryKind::NestingModeChanged { other, .. }
        | HistoryKind::NestingRemoved { other } => Some(*other),
        _ => None,
    }
}

fn cursor(seq: u64) -> String {
    format!("{seq:020}")
}

impl State {
    pub(crate) fn changes(&self, since: Option<&Place>, size: usize) -> Page<Change> {
        let Some(Place(from)) = since else {
            return Page {
                items: Vec::new(),
                next: Some(Place(cursor(self.seq))),
            };
        };
        let seen = |g: &ObjectId| !self.hidden(*g);
        let mut taken: Vec<&(u64, Change)> = self
            .changes
            .iter()
            .filter(|(n, c)| cursor(*n) > *from && groups_of_change(&c.kind).iter().all(seen))
            .collect();
        taken.truncate(size.max(1));
        let next = taken
            .last()
            .map_or_else(|| from.clone(), |(n, _)| cursor(*n));
        Page {
            items: taken.into_iter().map(|(_, c)| c.clone()).collect(),
            next: Some(Place(next)),
        }
    }

    pub(crate) fn history(
        &self,
        group: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<HistoryEntry>> {
        self.visible(group)?;
        self.need(group, Right::Read)?;
        let items = self
            .history
            .iter()
            .filter(|(_, g, e)| *g == group && other_of(&e.kind).is_none_or(|o| !self.hidden(o)))
            .map(|(n, _, e)| (cursor(*n), e.clone()))
            .collect();
        Ok(page(items, after, size))
    }
}
