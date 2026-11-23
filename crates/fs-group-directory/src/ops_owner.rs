//! Owners (ADR 0095): `create_owned`, `owners`, `my_level`, `appoint_owner`, `demote_owner`, `transfer_ownership`,
//! `resolve_owner`, and `nest_default`.
//!
//! Pattern: Command handlers on the state (visible, then the level, then the revision, then the input, then write and
//! log). The owner role is written only here (and by `create_owned`), never over the v1 role book.

use crate::groups_v1::shared::{ObjectId, PersonArt, PersonRef, Revision};
use crate::groups_v1::{
    Error, Field, Group, GroupDraft, HistoryKind, Membership, NestOutcome, Right,
};
use crate::groups_v2::{Error2, Level, Result2, role_owner};
use crate::ids::id_of_reference;
use crate::state::State;

impl State {
    /// The group is visible to the asker (unknown or hidden: `NotFound`) and she may read it (else `Withheld`).
    pub(crate) fn v2_gate(&self, g: ObjectId) -> Result2<()> {
        self.visible(g)?;
        if self.v2_readable(g) {
            Ok(())
        } else {
            Err(Error2::Withheld { group: g })
        }
    }

    /// The asker must be an owner of `g`, else `NeedsOwner` (an intervention is no ownership).
    fn need_owner(&self, g: ObjectId) -> Result2<()> {
        if self.is_owner(g, &self.asker) {
            Ok(())
        } else {
            Err(Error2::NeedsOwner { group: g })
        }
    }

    /// The person may become an owner: a known, direct-capable human (no AI, no unknown, no profile, no group).
    fn check_owner_candidate(&self, p: &PersonRef) -> Result2<()> {
        if matches!(
            p.art,
            PersonArt::Group | PersonArt::Organization | PersonArt::Profile
        ) {
            return Err(Error::WrongArt {
                field: Field::Person,
                got: p.art,
            }
            .into());
        }
        self.check_person(p)?;
        match self.people.status(p) {
            Some(false) => Ok(()),
            Some(true) => Err(Error::AgentNotAdmin.into()),
            None => Err(Error::AiStatusUnknown.into()),
        }
    }

    pub(crate) fn create_owned(&mut self, d: GroupDraft) -> Result2<Group> {
        let g = self.create(d)?;
        let me = self.asker.clone();
        self.join(g.id, &me, Some(&role_owner()), &me, None);
        Ok(g)
    }

    pub(crate) fn owners(&self, g: ObjectId) -> Result2<Vec<Membership>> {
        self.v2_gate(g)?;
        Ok(self.owner_memberships(g))
    }

    pub(crate) fn my_level(&self, g: ObjectId) -> Result2<Level> {
        self.visible(g)?;
        Ok(self.level_of(g, &self.asker))
    }

    /// Set the owner role on a direct membership of `person`, or add one.
    fn make_owner(&mut self, g: ObjectId, person: &PersonRef) -> Membership {
        let owner = role_owner();
        let me = self.asker.clone();
        let direct = self
            .members
            .iter()
            .position(|m| m.group == g && &m.person == person && m.role.is_none());
        let Some(at) = direct else {
            return self.join(g, person, Some(&owner), &me, None);
        };
        self.tick();
        let row = &mut self.members[at];
        row.role = Some(owner.clone());
        row.revision = Revision(row.revision.0 + 1);
        let done = row.clone();
        self.log(
            g,
            HistoryKind::RoleChanged {
                person: person.clone(),
                from: None,
                to: Some(owner),
            },
        );
        done
    }

    pub(crate) fn appoint_owner(&mut self, g: ObjectId, person: &PersonRef) -> Result2<Membership> {
        self.v2_gate(g)?;
        self.need_owner(g)?;
        self.check_owner_candidate(person)?;
        if let Some(m) = self
            .owner_memberships(g)
            .into_iter()
            .find(|m| &m.person == person)
        {
            return Err(Error::AlreadyThere(m.id).into());
        }
        Ok(self.make_owner(g, person))
    }

    pub(crate) fn demote_owner(&mut self, id: ObjectId, expected: Revision) -> Result2<Membership> {
        let m = self
            .members
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .ok_or(Error::NotFound(crate::groups_v1::Missing::Membership(id)))?;
        self.visible(m.group)
            .map_err(|_| Error::NotFound(crate::groups_v1::Missing::Membership(id)))?;
        self.need_owner(m.group)?;
        Self::cas(m.revision, expected)?;
        if m.role.as_ref() != Some(&role_owner()) {
            return Err(Error2::NotYourMembership(id));
        }
        if self.owner_memberships(m.group).len() <= 1 {
            return Err(Error2::LastOwner {
                group: m.group,
                person: m.person,
            });
        }
        Ok(self.drop_owner_role(id))
    }

    /// The owner role goes away on membership `id` (checks done).
    fn drop_owner_role(&mut self, id: ObjectId) -> Membership {
        self.tick();
        let at = self.members.iter().position(|m| m.id == id).unwrap_or(0);
        let row = &mut self.members[at];
        row.role = None;
        row.revision = Revision(row.revision.0 + 1);
        let done = row.clone();
        self.log(
            done.group,
            HistoryKind::RoleChanged {
                person: done.person.clone(),
                from: Some(role_owner()),
                to: None,
            },
        );
        done
    }

    pub(crate) fn transfer_ownership(
        &mut self,
        from: ObjectId,
        expected: Revision,
        to: &PersonRef,
        keep: bool,
    ) -> Result2<()> {
        let m = self
            .members
            .iter()
            .find(|m| m.id == from)
            .cloned()
            .ok_or(Error::NotFound(crate::groups_v1::Missing::Membership(from)))?;
        self.visible(m.group)
            .map_err(|_| Error::NotFound(crate::groups_v1::Missing::Membership(from)))?;
        if m.person != self.asker || m.role.as_ref() != Some(&role_owner()) {
            return Err(Error2::NotYourMembership(from));
        }
        self.need_owner(m.group)?;
        Self::cas(m.revision, expected)?;
        self.check_owner_candidate(to)?;
        let target = self
            .members
            .iter()
            .find(|x| x.group == m.group && &x.person == to)
            .cloned()
            .ok_or_else(|| Error2::NotDirectMember {
                group: m.group,
                person: to.clone(),
            })?;
        if target.person != m.person && target.role.as_ref() != Some(&role_owner()) {
            self.make_owner(m.group, to);
        }
        if !keep && &m.person != to {
            self.drop_owner_role(from);
        }
        Ok(())
    }

    pub(crate) fn resolve_owner(&self, owner: &PersonRef) -> Result2<Vec<PersonRef>> {
        match owner.art {
            PersonArt::Person | PersonArt::Profile => Ok(vec![owner.clone()]),
            PersonArt::Group => {
                let missing =
                    || Error::NotFound(crate::groups_v1::Missing::Person(owner.id.clone()));
                let g = id_of_reference(&owner.id).ok_or_else(missing)?;
                self.v2_gate(g).map_err(|e| match e {
                    Error2::V1(_) => Error2::V1(missing()),
                    other => other,
                })?;
                Ok(self
                    .owner_memberships(g)
                    .into_iter()
                    .map(|m| m.person)
                    .collect())
            }
            PersonArt::Organization => Err(Error::WrongArt {
                field: Field::Holder,
                got: owner.art,
            }
            .into()),
        }
    }

    pub(crate) fn nest_default(
        &mut self,
        child: ObjectId,
        parent: ObjectId,
    ) -> Result2<NestOutcome> {
        let mode = self.visible(parent)?.default_mode;
        Ok(self.request_nesting(child, parent, mode)?)
    }

    /// Whether the asker holds at least `level` in `g`, else `NeedsLevel`.
    pub(crate) fn need_level(&self, g: ObjectId, level: Level) -> Result2<()> {
        let have = self.level_of(g, &self.asker);
        let via_iv = level <= Level::Manager && self.may_ask(g, Right::Manage);
        if have >= level || via_iv {
            Ok(())
        } else {
            Err(Error2::NeedsLevel { group: g, level })
        }
    }
}
