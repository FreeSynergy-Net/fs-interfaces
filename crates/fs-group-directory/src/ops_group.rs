//! Reading groups: `list`, `get`, `get_by_reference`, and the summary row.
//!
//! Pattern: Repository query (filters over the group table; the AI visibility rule filters first).

use crate::groups_v1::shared::{ObjectId, Page, Place, Reference};
use crate::groups_v1::{Group, GroupQuery, GroupSummary, ParentFilter, Result};
use crate::ids::id_of_reference;
use crate::paging::page;
use crate::state::{Row, State};

impl State {
    /// One list row; counts only visible neighbours, numbers only where the member list may be seen.
    pub(crate) fn summary(&self, row: &Row) -> GroupSummary {
        let g = &row.group;
        let count = |own_child: bool| {
            let n = self
                .edges
                .iter()
                .filter(|e| {
                    if own_child {
                        e.child == g.id
                    } else {
                        e.parent == g.id
                    }
                })
                .filter(|e| !self.hidden(if own_child { e.parent } else { e.child }))
                .count();
            u32::try_from(n).unwrap_or(u32::MAX)
        };
        let direct = self.members.iter().filter(|m| m.group == g.id).count();
        GroupSummary {
            id: g.id,
            reference: g.reference.clone(),
            revision: g.revision,
            art: g.art,
            name: g.name.clone(),
            kind: g.kind.clone(),
            default_mode: g.default_mode,
            parents: count(true),
            children: count(false),
            direct_members: self
                .can_see_members(g.id)
                .then(|| u32::try_from(direct).unwrap_or(u32::MAX)),
            sensitive: g.sensitive,
            ai: g.ai,
            mirror: g.mirror,
            restorable_until: row.deleted,
        }
    }

    pub(crate) fn list(
        &self,
        q: &GroupQuery,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<GroupSummary>> {
        let children_of = match q.parent {
            ParentFilter::ChildrenOf(id) => {
                self.visible(id)?;
                Some(id)
            }
            _ => None,
        };
        let needle = q.text.to_lowercase();
        let mut items = Vec::new();
        for row in self.groups.values() {
            let g = &row.group;
            let text_ok = needle.is_empty()
                || g.name.to_lowercase().contains(&needle)
                || g.kind
                    .as_ref()
                    .is_some_and(|k| k.to_lowercase().contains(&needle));
            let parent_ok = match q.parent {
                ParentFilter::Any => true,
                ParentFilter::Roots => !self
                    .edges
                    .iter()
                    .any(|e| e.child == g.id && !self.hidden(e.parent)),
                ParentFilter::ChildrenOf(_) => self
                    .edges
                    .iter()
                    .any(|e| e.child == g.id && Some(e.parent) == children_of),
            };
            let member_ok = q.member.as_ref().is_none_or(|p| {
                self.can_see_members(g.id)
                    && self
                        .members
                        .iter()
                        .any(|m| m.group == g.id && &m.person == p)
            });
            if text_ok
                && parent_ok
                && member_ok
                && !self.hidden(g.id)
                && (row.deleted.is_none() || q.include_deleted)
            {
                items.push((
                    format!("{}\0{}", g.name.to_lowercase(), key_of(g.id)),
                    self.summary(row),
                ));
            }
        }
        Ok(page(items, after, size))
    }

    pub(crate) fn get(&self, id: ObjectId) -> Option<Group> {
        self.visible(id).ok().cloned()
    }

    pub(crate) fn get_by_reference(&self, r: &Reference) -> Option<Group> {
        self.get(id_of_reference(r)?)
    }
}

/// A sortable text of an id.
pub(crate) fn key_of(id: ObjectId) -> String {
    format!("{:020}", crate::ids::object_number(id))
}
