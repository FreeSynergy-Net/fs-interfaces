//! Invitations: `invitations`, `my_invitations`, `invite`, `answer_invitation`, `revoke_invitation`.
//!
//! Pattern: State (the life of an invitation; an invitation past its date reads as `Expired`) + Command handlers.

use crate::groups_v1::shared::{ObjectId, PersonRef, Revision, Utc};
use crate::groups_v1::{
    ChangeKind, Error, Field, FieldProblem, HistoryKind, Invitation, InvitationAnswer,
    InvitationOutcome, InvitationState, Missing, ProblemKind, Result, Right, RoleRef,
};
use crate::state::State;

impl State {
    /// The invitation as it reads now: past its date, a pending one is `Expired`.
    fn invite_view(&self, i: &Invitation) -> Invitation {
        let late = i.state == InvitationState::Pending && i.expires.is_some_and(|t| t <= self.now);
        Invitation {
            state: if late {
                InvitationState::Expired
            } else {
                i.state
            },
            ..i.clone()
        }
    }

    pub(crate) fn invitations(&self, group: ObjectId) -> Result<Vec<Invitation>> {
        self.visible(group)?;
        self.need(group, Right::Read)?;
        let mut out: Vec<Invitation> = self
            .invites
            .iter()
            .filter(|i| i.group == group)
            .map(|i| self.invite_view(i))
            .collect();
        out.sort_by_key(|i| std::cmp::Reverse((i.sent, i.id)));
        Ok(out)
    }

    pub(crate) fn my_invitations(&self) -> Vec<Invitation> {
        let mut out: Vec<Invitation> = self
            .invites
            .iter()
            .filter(|i| i.invitee == self.asker && self.visible(i.group).is_ok())
            .map(|i| self.invite_view(i))
            .filter(|i| i.state == InvitationState::Pending)
            .collect();
        out.sort_by_key(|i| std::cmp::Reverse((i.sent, i.id)));
        out
    }

    pub(crate) fn invite(
        &mut self,
        group: ObjectId,
        invitee: &PersonRef,
        role: Option<&RoleRef>,
        expires: Option<Utc>,
    ) -> Result<Invitation> {
        let g = self.visible(group)?.clone();
        self.need(group, Right::AddMembers)?;
        self.check_person(invitee)?;
        self.check_kind(&g, invitee, role)?;
        if expires.is_some_and(|t| t <= self.now) {
            return Err(Error::Invalid(vec![FieldProblem {
                field: Field::Expires,
                problem: ProblemKind::InPast,
            }]));
        }
        if let Some(m) = self
            .members
            .iter()
            .find(|m| m.group == group && &m.person == invitee && m.role.as_ref() == role)
        {
            return Err(Error::AlreadyThere(m.id));
        }
        let open = self.invites.iter().map(|i| self.invite_view(i)).find(|i| {
            i.group == group
                && &i.invitee == invitee
                && i.role.as_ref() == role
                && i.state == InvitationState::Pending
        });
        if let Some(i) = open {
            return Err(Error::AlreadyThere(i.id));
        }
        let now = self.tick();
        let inv = Invitation {
            id: self.fresh(),
            revision: Revision(1),
            group,
            invitee: invitee.clone(),
            role: role.cloned(),
            state: InvitationState::Pending,
            expires,
            invited_by: self.asker.clone(),
            sent: now,
            answered: None,
        };
        self.invites.push(inv.clone());
        self.log(
            group,
            HistoryKind::Invited {
                invitee: invitee.clone(),
            },
        );
        self.change(ChangeKind::Invitation { group });
        Ok(inv)
    }

    /// The invitation by id; one of a hidden or deleted group is `NotFound`.
    fn invite_by_id(&self, id: ObjectId) -> Result<Invitation> {
        let missing = || Error::NotFound(Missing::Invitation(id));
        let i = self
            .invites
            .iter()
            .find(|i| i.id == id)
            .ok_or_else(missing)?;
        self.visible(i.group).map_err(|_| missing())?;
        Ok(i.clone())
    }

    /// Store a new state of an invitation and log it.
    fn close_invite(&mut self, id: ObjectId, state: InvitationState) -> Result<Invitation> {
        let now = self.now;
        let i = self
            .invites
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(Error::NotFound(Missing::Invitation(id)))?;
        i.state = state;
        i.revision = Revision(i.revision.0 + 1);
        i.answered = Some(now);
        let done = i.clone();
        self.log(
            done.group,
            HistoryKind::InvitationClosed {
                invitee: done.invitee.clone(),
                state,
            },
        );
        self.change(ChangeKind::Invitation { group: done.group });
        Ok(done)
    }

    pub(crate) fn answer_invitation(
        &mut self,
        id: ObjectId,
        expected: Revision,
        answer: InvitationAnswer,
    ) -> Result<InvitationOutcome> {
        let i = self.invite_by_id(id)?;
        if i.invitee != self.asker {
            return Err(Error::NotInvitee);
        }
        Self::cas(i.revision, expected)?;
        self.tick();
        let seen = self.invite_view(&i);
        if seen.state == InvitationState::Expired && i.state == InvitationState::Pending {
            self.close_invite(id, InvitationState::Expired)?;
            return Err(Error::Expired {
                at: i.expires.unwrap_or(self.now),
            });
        }
        if seen.state != InvitationState::Pending {
            return Err(Error::InvitationClosed(seen.state));
        }
        if answer == InvitationAnswer::Decline {
            return self
                .close_invite(id, InvitationState::Declined)
                .map(InvitationOutcome::Declined);
        }
        let g = self.alive(i.group)?.clone();
        self.check_kind(&g, &i.invitee, i.role.as_ref())?;
        let held = self
            .members
            .iter()
            .find(|m| m.group == i.group && m.person == i.invitee && m.role == i.role)
            .cloned();
        let joined = match held {
            Some(m) => crate::groups_v1::Membership {
                in_force: self.member_force(&m),
                ..m
            },
            None => self.join(
                i.group,
                &i.invitee,
                i.role.as_ref(),
                &i.invited_by,
                Some(id),
            ),
        };
        self.close_invite(id, InvitationState::Accepted)?;
        Ok(InvitationOutcome::Joined(joined))
    }

    pub(crate) fn revoke_invitation(
        &mut self,
        id: ObjectId,
        expected: Revision,
    ) -> Result<Invitation> {
        let i = self.invite_by_id(id)?;
        if i.invited_by != self.asker {
            self.need(i.group, Right::RemoveMembers)?;
        }
        Self::cas(i.revision, expected)?;
        let seen = self.invite_view(&i);
        if seen.state != InvitationState::Pending {
            return Err(Error::InvitationClosed(seen.state));
        }
        self.tick();
        self.close_invite(id, InvitationState::Revoked)
    }
}
