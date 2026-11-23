//! Scope and the operator's read log in the fake.
//!
//! Pattern: Guard (rights decide what a scope shows; the operator's reads leave an entry for the owner).

use super::access::{Actor, Mode};
use super::state::{Fault, Node, World};
use crate::entry::{Change, ChangeKind, Entry, PageReq};
use crate::error::{Error, Result};
use crate::ids::EntryId;
use fs_types::{PageCursor, Problem};

impl World {
    /// Every entry of one generator the asker may read, newest first, paged.
    pub(super) fn scope_of(
        &self,
        a: &Actor,
        generator: &str,
        page: &PageReq,
    ) -> Result<(Vec<Entry>, Option<PageCursor>)> {
        if self.is_bot(a) {
            if a.fault == Some(Fault::BotScopeEmpty) {
                return Ok((Vec::new(), None));
            }
            return Err(Error::Forbidden {
                capability: "storage.read".to_owned(),
            });
        }
        if page.size == 0 || page.size > 1000 {
            return Err(Error::Invalid(vec![Problem {
                field: "size".into(),
                reason: "range".into(),
            }]));
        }
        let mut hits: Vec<&Node> = self
            .nodes
            .values()
            .filter(|n| {
                (n.trashed_from.is_none() || a.fault == Some(Fault::ScopeTrash))
                    && n.entry.generator.as_deref() == Some(generator)
            })
            .filter(|n| {
                a.fault == Some(Fault::ScopeLeak) || self.allowed(a, &n.entry.owner, Mode::Read)
            })
            .collect();
        hits.sort_by_key(|n| (std::cmp::Reverse(n.entry.modified), n.entry.name.clone()));
        let start = page
            .cursor
            .as_ref()
            .and_then(|c| c.0.parse::<usize>().ok())
            .unwrap_or(0);
        let end = start.saturating_add(page.size).min(hits.len());
        let out = hits
            .get(start..end)
            .unwrap_or(&[])
            .iter()
            .map(|n| Self::present(a, n))
            .collect();
        Ok((out, (end < hits.len()).then(|| PageCursor(end.to_string()))))
    }

    /// Decision 16.2: when the operator opens or copies foreign content, the owner gets an entry. Information only: it
    /// records access through the program and protects nothing (whoever is root can read past it).
    pub(super) fn log_operator_read(&mut self, a: &Actor, id: &EntryId) {
        let Some(n) = self.nodes.get(id.as_str()) else {
            return;
        };
        if (self.is_operator(a) || a.fault == Some(Fault::LogAll))
            && n.entry.owner.id != a.who.id
            && a.fault != Some(Fault::NoOperatorLog)
        {
            let owner = n.entry.owner.clone();
            self.log.push((
                Change {
                    id: id.clone(),
                    kind: ChangeKind::Viewed,
                },
                owner,
            ));
        }
    }
}
