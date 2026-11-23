//! Members: `members`, `effective_members`, `groups_of`, `add_member`, `set_role`, `remove_member`.
//!
//! Pattern: Repository query and Command handlers (the AI kind rule is checked on every way a member gets in).

use crate::access::Eff;
use crate::groups_v1::shared::{
    ObjectId, Page, PersonArt, PersonRef, Place, Revision, TypeProblem,
};
use crate::groups_v1::{
    ChangeKind, EffectiveGroup, EffectiveMember, Error, Field, FieldProblem, Group, HistoryKind,
    InForce, Inherited, Membership, Missing, ProblemKind, Result, Right, RoleRef,
};
use crate::ids::key;
use crate::paging::page;
use crate::state::State;

impl State {
    /// The member list of a visible group, `Forbidden{Read}` when the asker may not see it.
    pub(crate) fn members_open(&self, group: ObjectId) -> Result<&Group> {
        let g = self.visible(group)?;
        if self.can_see_members(group) {
            Ok(g)
        } else {
            Err(Error::Forbidden {
                group,
                right: Right::Read,
            })
        }
    }

    /// A membership as the asker reads it (the force is computed now).
    fn member_view(&self, m: &Membership) -> Membership {
        Membership {
            in_force: self.member_force(m),
            ..m.clone()
        }
    }

    pub(crate) fn members(
        &self,
        group: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<Membership>> {
        self.members_open(group)?;
        let items = self
            .members
            .iter()
            .filter(|m| m.group == group)
            .map(|m| {
                (
                    format!("{:020}{}", m.since.millis(), crate::ops_group::key_of(m.id)),
                    self.member_view(m),
                )
            })
            .collect();
        Ok(page(items, after, size))
    }

    /// The origins as the asker may see them (`from` and `via` hide what she may not read).
    fn inherited_view(&self, raw: &[(ObjectId, Vec<ObjectId>)]) -> Vec<Inherited> {
        raw.iter()
            .map(|(src, via)| Inherited {
                from: self.readable(*src).then_some(*src),
                via: if via.iter().all(|g| self.readable(*g)) {
                    via.clone()
                } else {
                    Vec::new()
                },
            })
            .collect()
    }

    pub(crate) fn effective_members(
        &self,
        group: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<EffectiveMember>> {
        self.members_open(group)?;
        let items = self
            .effective_raw(group)
            .into_iter()
            .map(|e: Eff| {
                (
                    key(&e.person),
                    EffectiveMember {
                        person: e.person,
                        direct: e.direct,
                        inherited: self.inherited_view(&e.inherited),
                    },
                )
            })
            .collect();
        Ok(page(items, after, size))
    }

    /// A group or an organisation is never a person.
    pub(crate) fn check_art(p: &PersonRef) -> Result<()> {
        if matches!(p.art, PersonArt::Group | PersonArt::Organization) {
            return Err(Error::WrongArt {
                field: Field::Person,
                got: p.art,
            });
        }
        Ok(())
    }

    /// The person must be a person or profile and known to people.
    pub(crate) fn check_person(&self, p: &PersonRef) -> Result<()> {
        Self::check_art(p)?;
        if self.people.knows(p) {
            Ok(())
        } else {
            Err(Error::NotFound(Missing::Person(p.id.clone())))
        }
    }

    /// The AI kind rule for a new member (or invitee) and the admin-role rule for the role.
    pub(crate) fn check_kind(
        &self,
        g: &Group,
        p: &PersonRef,
        role: Option<&RoleRef>,
    ) -> Result<()> {
        let status = self.people.status(p);
        // An AI account is never a member (no AI groups, ADR 0091): `AiNesting` names the refusal.
        if status == Some(true) {
            return Err(Error::AiNesting {
                group: g.id,
                path: vec![g.id],
            });
        }
        if let Some(role) = role {
            let info = self
                .people
                .role_info(role)
                .ok_or(Error::Invalid(vec![FieldProblem {
                    field: Field::Role,
                    problem: ProblemKind::BadReference(TypeProblem::NotInList),
                }]))?;
            match status {
                Some(true) if info.admin => return Err(Error::AgentNotAdmin),
                None if info.admin => return Err(Error::AiStatusUnknown),
                _ => {}
            }
        }
        Ok(())
    }

    pub(crate) fn add_member(
        &mut self,
        group: ObjectId,
        person: &PersonRef,
        role: Option<&RoleRef>,
    ) -> Result<Membership> {
        let g = self.visible(group)?.clone();
        self.need(group, Right::AddMembers)?;
        self.check_person(person)?;
        self.check_kind(&g, person, role)?;
        if let Some(m) = self
            .members
            .iter()
            .find(|m| m.group == group && &m.person == person && m.role.as_ref() == role)
        {
            return Err(Error::AlreadyThere(m.id));
        }
        let me = self.asker.clone();
        Ok(self.join(group, person, role, &me, None))
    }

    /// Put a membership in (checks done).
    pub(crate) fn join(
        &mut self,
        group: ObjectId,
        person: &PersonRef,
        role: Option<&RoleRef>,
        by: &PersonRef,
        from: Option<ObjectId>,
    ) -> Membership {
        let now = self.tick();
        let m = Membership {
            id: self.fresh(),
            revision: Revision(1),
            group,
            person: person.clone(),
            role: role.cloned(),
            since: now,
            added_by: by.clone(),
            from_invitation: from,
            in_force: InForce::Yes,
        };
        self.members.push(m.clone());
        self.log(
            group,
            HistoryKind::MemberAdded {
                person: person.clone(),
                role: role.cloned(),
            },
        );
        self.change(ChangeKind::Membership { group });
        self.member_view(&m)
    }

    /// The membership by id; one of a hidden or deleted group is `NotFound`.
    fn member_by_id(&self, id: ObjectId) -> Result<Membership> {
        let missing = || Error::NotFound(Missing::Membership(id));
        let m = self
            .members
            .iter()
            .find(|m| m.id == id)
            .ok_or_else(missing)?;
        self.visible(m.group).map_err(|_| missing())?;
        Ok(m.clone())
    }

    pub(crate) fn set_role(
        &mut self,
        id: ObjectId,
        expected: Revision,
        role: Option<&RoleRef>,
    ) -> Result<Membership> {
        let m = self.member_by_id(id)?;
        self.need(m.group, Right::Manage)?;
        self.guard_owner_membership(&m)?;
        Self::cas(m.revision, expected)?;
        let g = self.alive(m.group)?.clone();
        self.check_kind(&g, &m.person, role)?;
        if m.role.as_ref() == role {
            return Ok(self.member_view(&m));
        }
        self.tick();
        let mut done = m.clone();
        if let Some(row) = self.members.iter_mut().find(|x| x.id == id) {
            row.role = role.cloned();
            row.revision = Revision(m.revision.0 + 1);
            done = row.clone();
        }
        self.log(
            m.group,
            HistoryKind::RoleChanged {
                person: m.person,
                from: m.role,
                to: role.cloned(),
            },
        );
        self.change(ChangeKind::Membership { group: m.group });
        Ok(self.member_view(&done))
    }

    pub(crate) fn remove_member(&mut self, id: ObjectId, expected: Revision) -> Result<()> {
        let m = self.member_by_id(id)?;
        if m.person != self.asker {
            self.need(m.group, Right::RemoveMembers)?;
        }
        self.guard_owner_membership(&m)?;
        Self::cas(m.revision, expected)?;
        let all: Vec<ObjectId> = self.groups.keys().copied().collect();
        if all
            .into_iter()
            .any(|g| self.would_orphan(g, Some(id), None))
        {
            return Err(Error::LastManager);
        }
        self.tick();
        self.members.retain(|x| x.id != id);
        self.log(m.group, HistoryKind::MemberRemoved { person: m.person });
        self.change(ChangeKind::Membership { group: m.group });
        Ok(())
    }

    pub(crate) fn groups_of(&self, person: &PersonRef) -> Result<Vec<EffectiveGroup>> {
        Self::check_art(person)?;
        let mut out = Vec::new();
        for (g, row) in &self.groups {
            if row.deleted.is_some()
                || self.hidden(*g)
                || !(person == &self.asker || self.can_see_members(*g))
            {
                continue;
            }
            if let Some(e) = self
                .effective_raw(*g)
                .into_iter()
                .find(|e| &e.person == person)
            {
                out.push(EffectiveGroup {
                    group: *g,
                    direct: e.direct,
                    inherited: self.inherited_view(&e.inherited),
                });
            }
        }
        Ok(out)
    }
}
