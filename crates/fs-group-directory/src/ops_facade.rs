//! The narrow view for other programs (`GroupMembers`): describe a group, resolve it to persons, list the groups of
//! a person.
//!
//! Pattern: Facade (a small read view over the same state; the visibility rules of `Groups` apply unchanged).

use crate::groups::shared::{PersonArt, PersonRef};
use crate::groups::{Error, GroupCard, Missing, Result};
use crate::ids::{group_ref, id_of_reference};
use crate::state::State;

impl State {
    pub(crate) fn card(&self, r: &PersonRef) -> Option<GroupCard> {
        if !matches!(r.art, PersonArt::Group | PersonArt::Organization) {
            return None;
        }
        let row = self.groups.get(&id_of_reference(&r.id)?)?;
        let g = &row.group;
        if self.hidden(g.id) {
            return None;
        }
        let count = (row.deleted.is_none() && self.can_see_members(g.id))
            .then(|| u32::try_from(self.effective_raw(g.id).len()).unwrap_or(u32::MAX));
        Some(GroupCard {
            group: r.clone(),
            name: g.name.clone(),
            effective_members: count,
            deleted: row.deleted.is_some(),
        })
    }

    pub(crate) fn members_of(&self, r: &PersonRef) -> Result<Vec<PersonRef>> {
        if !matches!(r.art, PersonArt::Group | PersonArt::Organization) {
            return Err(Error::WrongArt {
                field: crate::groups::Field::Person,
                got: r.art,
            });
        }
        let id =
            id_of_reference(&r.id).ok_or_else(|| Error::NotFound(Missing::Person(r.id.clone())))?;
        self.members_open(id)?;
        Ok(self
            .effective_raw(id)
            .into_iter()
            .map(|e| e.person)
            .collect())
    }

    pub(crate) fn memberships_of(&self, person: &PersonRef) -> Result<Vec<PersonRef>> {
        let mut out = Vec::new();
        for g in self.groups_of(person)? {
            out.push(group_ref(g.group));
        }
        Ok(out)
    }
}
