//! The v2 tables of a `FakeGroups` (addons, scope grants, interventions) and the questions on them: who is an owner,
//! which level has the asker, which intervention is open.
//!
//! Pattern: Specification (levels and interventions as questions on the state; `decide` in `access.rs` asks them).

use crate::groups_v1::shared::{ObjectId, PersonRef, Utc};
use crate::groups_v1::{Membership, RoleRef};
use crate::groups_v2::{Addon, Intervention, Level, ScopeGrant, role_manager, role_owner};
use crate::state::State;

/// The tables only v2 knows.
#[derive(Default)]
pub(crate) struct V2State {
    pub addons: Vec<Addon>,
    pub scopes: Vec<ScopeGrant>,
    pub interventions: Vec<Intervention>,
}

impl State {
    /// Whether `m` carries `role` and counts (a human; an AI or unknown never holds owner or manager).
    fn holds_role(&self, m: &Membership, role: &RoleRef) -> bool {
        m.role.as_ref() == Some(role) && self.people.status(&m.person) == Some(false)
    }

    /// The living owner memberships of `g`, oldest first.
    pub(crate) fn owner_memberships(&self, g: ObjectId) -> Vec<Membership> {
        let owner = role_owner();
        let mut out: Vec<Membership> = self
            .members
            .iter()
            .filter(|m| m.group == g && self.holds_role(m, &owner))
            .cloned()
            .collect();
        out.sort_by_key(|m| (m.since, m.id));
        out
    }

    /// Whether `who` is an owner of `g` (direct member with the role `owner`, a human).
    pub(crate) fn is_owner(&self, g: ObjectId, who: &PersonRef) -> bool {
        self.owner_memberships(g).iter().any(|m| &m.person == who)
    }

    /// The membership of `who` in `g` that gives `Manage` through a role (owner or manager).
    pub(crate) fn role_membership(&self, g: ObjectId, who: &PersonRef) -> Option<&Membership> {
        let (owner, manager) = (role_owner(), role_manager());
        self.members.iter().find(|m| {
            m.group == g
                && &m.person == who
                && (self.holds_role(m, &owner) || self.holds_role(m, &manager))
        })
    }

    /// Humans who hold `Manage` through a role at `g` (counted by `manage_holders`).
    pub(crate) fn role_holders(&self, g: ObjectId) -> Vec<PersonRef> {
        let (owner, manager) = (role_owner(), role_manager());
        self.members
            .iter()
            .filter(|m| {
                m.group == g && (self.holds_role(m, &owner) || self.holds_role(m, &manager))
            })
            .map(|m| m.person.clone())
            .collect()
    }

    /// The level of `who` in `g` (owner, manager, direct member, none); an intervention is not a level.
    pub(crate) fn level_of(&self, g: ObjectId, who: &PersonRef) -> Level {
        if self.is_owner(g, who) {
            Level::Owner
        } else if self
            .decide_base(g, who, crate::groups_v1::Right::Manage)
            .allowed
        {
            Level::Manager
        } else if self
            .members
            .iter()
            .any(|m| m.group == g && &m.person == who)
        {
            Level::Member
        } else {
            Level::None
        }
    }

    /// The open intervention of `who` into `g` right now (not closed, not past its end).
    pub(crate) fn open_iv(&self, g: ObjectId, who: &PersonRef) -> Option<&Intervention> {
        let now: Utc = self.now;
        self.v2
            .interventions
            .iter()
            .find(|i| i.group == g && &i.actor == who && i.closed.is_none() && now < i.ends)
    }

    /// Whether the asker may read what v2 lists about `g` (owners, addons, interventions): the same view as the
    /// group itself; a sensitive group without an insight is `Withheld`.
    pub(crate) fn v2_readable(&self, g: ObjectId) -> bool {
        self.readable(g)
    }
}

impl State {
    /// A log entry written while the asker acts ONLY through an open intervention (no right of her own) is also an
    /// act of that intervention: visible to every reader of the group (ADR 0095 Pkt. 7, no silent way).
    pub(crate) fn note_intervention(&mut self, g: ObjectId, kind: crate::groups_v1::HistoryKind) {
        self.note_act(g, crate::groups_v2::ActKind::History(kind));
    }

    /// An addon change (attach, update, detach): in the change stream, and an act of the open intervention when the
    /// asker has no right of her own (the same rule as `note_intervention`).
    pub(crate) fn note_addon(
        &mut self,
        g: ObjectId,
        verb: crate::groups_v2::AddonVerb,
        iri: crate::groups_v1::shared::Reference,
    ) {
        self.change(crate::groups_v1::ChangeKind::Group(g));
        self.note_act(g, crate::groups_v2::ActKind::Addon { verb, iri });
    }

    fn note_act(&mut self, g: ObjectId, kind: crate::groups_v2::ActKind) {
        let who = self.asker.clone();
        if self
            .decide_base(g, &who, crate::groups_v1::Right::Manage)
            .allowed
        {
            return;
        }
        let now = self.now;
        if let Some(i) = self
            .v2
            .interventions
            .iter_mut()
            .find(|i| i.group == g && i.actor == who && i.closed.is_none() && now < i.ends)
        {
            i.acts
                .push(crate::groups_v2::InterventionAct { at: now, kind });
        }
    }
}

impl State {
    /// Taking an owner membership away or changing its role (v1 paths `remove_member`, `set_role`): only an owner may,
    /// and never the last one (v1 error `LastManager`; the v2 paths say `NeedsOwner` and `LastOwner`).
    pub(crate) fn guard_owner_membership(&self, m: &Membership) -> crate::groups_v1::Result<()> {
        use crate::groups_v1::{Error, Right};
        if m.role.as_ref() != Some(&role_owner()) {
            return Ok(());
        }
        if !self.is_owner(m.group, &self.asker) {
            return Err(Error::Forbidden {
                group: m.group,
                right: Right::Manage,
            });
        }
        if self.owner_memberships(m.group).len() <= 1 {
            return Err(Error::LastManager);
        }
        Ok(())
    }

    /// Giving a role over a v1 path (`add_member`, `set_role`, `invite`): the owner role is only made by the v2
    /// operations (`create_owned`, `appoint_owner`, `transfer_ownership`), the manager role needs `Manage`
    /// (ADR 0095 Pkt. 3). `granting` is false when the invitee accepts (the right was checked at `invite`).
    pub(crate) fn guard_role_grant(
        &self,
        g: ObjectId,
        role: Option<&RoleRef>,
        granting: bool,
    ) -> crate::groups_v1::Result<()> {
        use crate::groups_v1::{Error, Right};
        match role {
            Some(r) if *r == role_owner() => Err(Error::Forbidden {
                group: g,
                right: Right::Manage,
            }),
            Some(r) if granting && *r == role_manager() => self.need(g, Right::Manage),
            _ => Ok(()),
        }
    }

    /// Deleting a group that has an owner needs `Own` or an intervention (ADR 0095 Pkt. 11); without owners `Manage`
    /// is enough (checked by the caller).
    pub(crate) fn guard_delete_with_owner(&self, g: ObjectId) -> crate::groups_v1::Result<()> {
        use crate::groups_v1::{Error, Right};
        let me = &self.asker;
        if self.owner_memberships(g).is_empty()
            || self.is_owner(g, me)
            || self.open_iv(g, me).is_some()
        {
            return Ok(());
        }
        Err(Error::Forbidden {
            group: g,
            right: Right::Manage,
        })
    }
}
