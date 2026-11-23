//! The mutant `PathTraversal`: a path walk that interprets `..` and empty segments (what the guard must prevent).
//!
//! Pattern: Mutant (a deliberately wrong version of `resolve_by_list`, only reachable with the fault switched on).

use super::access::Actor;
use super::state::World;
use crate::entry::Entry;
use crate::error::{Error, Result};
use crate::ids::EntryId;

impl World {
    pub(super) fn naive_path(&self, a: &Actor, root: &EntryId, path: &str) -> Result<Entry> {
        let mut cur = root.as_str().to_owned();
        for seg in path.split('/') {
            match seg {
                "" | "." => {}
                ".." => {
                    cur = self
                        .nodes
                        .get(&cur)
                        .and_then(|n| n.parent.clone())
                        .unwrap_or(cur);
                }
                name => {
                    cur = self
                        .nodes
                        .iter()
                        .find(|(_, n)| {
                            n.parent.as_deref() == Some(cur.as_str()) && n.entry.name == name
                        })
                        .map(|(k, _)| k.clone())
                        .ok_or_else(|| Error::NotFound(EntryId(name.to_owned())))?;
                }
            }
        }
        self.stat_of(a, &EntryId(cur))
    }
}
