//! Who may do what, and who sees what: the AI visibility rule, the force of memberships and grants, `may_do`.
//!
//! Pattern: Specification (rights and visibility as questions on the state; the answer names its reason).

use crate::groups::shared::{ObjectId, PersonArt, PersonRef};
use crate::groups::{
    Decision, Error, Grant, InForce, Membership, Reason, Result, Right, RoleRef, Visibility,
};
use crate::ids::{id_of_reference, key};
use crate::nesting_logic::hull;
use crate::state::State;

/// One effective member with the way it got in (ids only; the asker's view hides some of them).
pub(crate) struct Eff {
    pub person: PersonRef,
    pub direct: Option<ObjectId>,
    pub inherited: Vec<(ObjectId, Vec<ObjectId>)>,
}

/// Whether a right held covers a right wanted (`Manage` covers all; every right covers `Read`).
fn covers(held: Right, wanted: Right) -> bool {
    held == wanted || held == Right::Manage || wanted == Right::Read
}

impl State {
    /// Whether the asker is marked as an AI.
    pub(crate) fn asker_is_agent(&self) -> bool {
        self.people.status(&self.asker) == Some(true)
    }

    /// A group with an admin role given to a member (the "admin right" of the rule).
    pub(crate) fn is_admin_group(&self, g: ObjectId) -> bool {
        self.members.iter().filter(|m| m.group == g).any(|m| {
            m.role
                .as_ref()
                .and_then(|r| self.people.role_info(r))
                .is_some_and(|i| i.admin)
        })
    }

    /// The AI visibility rule: an asking AI does not see admin and sensitive groups at all.
    pub(crate) fn hidden(&self, g: ObjectId) -> bool {
        self.asker_is_agent()
            && self
                .groups
                .get(&g)
                .is_some_and(|r| r.group.sensitive || self.is_admin_group(g))
    }

    /// Whether a membership counts (the account kind fits the group) and in which force.
    pub(crate) fn member_force(&self, m: &Membership) -> InForce {
        let status = self.people.status(&m.person);
        if status == Some(true) {
            return InForce::KindMismatch;
        }
        let admin = m
            .role
            .as_ref()
            .and_then(|r| self.people.role_info(r))
            .is_some_and(|i| i.admin);
        match (admin, status) {
            (true, None) => InForce::AiStatusUnknown,
            _ => InForce::Yes,
        }
    }

    /// The force of a grant (`Manage` and AI rules).
    pub(crate) fn grant_force(&self, g: &Grant) -> InForce {
        let closed = self.groups.get(&g.group).is_some_and(|r| r.group.sensitive)
            || self.is_admin_group(g.group);
        if g.holder.art == PersonArt::Group {
            return InForce::Yes;
        }
        match self.people.status(&g.holder) {
            Some(true) if g.right == Right::Manage || closed => InForce::KindMismatch,
            None if g.right == Right::Manage => InForce::AiStatusUnknown,
            _ => InForce::Yes,
        }
    }

    /// Effective members of `g` over the hull (`nesting.rs`), each person once.
    pub(crate) fn effective_raw(&self, g: ObjectId) -> Vec<Eff> {
        let mut out: Vec<Eff> = Vec::new();
        for src in hull(&self.logic_edges(), g) {
            for m in self.members.iter().filter(|m| m.group == src.group) {
                if self.member_force(m) == InForce::KindMismatch {
                    continue;
                }
                let at = out
                    .iter()
                    .position(|e| e.person == m.person)
                    .unwrap_or_else(|| {
                        out.push(Eff {
                            person: m.person.clone(),
                            direct: None,
                            inherited: Vec::new(),
                        });
                        out.len() - 1
                    });
                if src.group == g {
                    out[at].direct = Some(m.id);
                } else {
                    out[at].inherited.push((src.group, src.via.clone()));
                }
            }
        }
        out
    }

    /// Whether `who` is an effective member of `g`.
    pub(crate) fn holds_member(&self, g: ObjectId, who: &PersonRef) -> bool {
        self.effective_raw(g).iter().any(|e| &e.person == who)
    }

    /// Whether the asker may read `g` in paths and answers (not hidden; a sensitive group only with a view).
    pub(crate) fn readable(&self, g: ObjectId) -> bool {
        !self.hidden(g)
            && self
                .groups
                .get(&g)
                .is_some_and(|r| !r.group.sensitive || self.can_see_members(g))
    }

    /// Whether the asker may see the member list of `g` (`Visibility`).
    pub(crate) fn can_see_members(&self, g: ObjectId) -> bool {
        let Some(row) = self.groups.get(&g) else {
            return false;
        };
        let member = || self.holds_member(g, &self.asker) || self.may_ask(g, Right::Read);
        match (row.group.visibility, row.group.sensitive) {
            (Visibility::Managers, _) => self.may_ask(g, Right::Manage),
            (Visibility::Members, _) | (Visibility::Everyone, true) => member(),
            (Visibility::Everyone, false) => true,
        }
    }

    /// `may_do` for the asker, as a plain answer.
    pub(crate) fn may_ask(&self, g: ObjectId, right: Right) -> bool {
        self.decide(g, &self.asker, right).allowed
    }

    /// The asker needs `right` at `g`, else `Forbidden`.
    pub(crate) fn need(&self, g: ObjectId, right: Right) -> Result<()> {
        if self.may_ask(g, right) {
            Ok(())
        } else {
            Err(Error::Forbidden { group: g, right })
        }
    }

    /// The answer of `may_do` with its reason (the group is known to exist; no visibility rule here).
    pub(crate) fn decide(&self, g: ObjectId, who: &PersonRef, right: Right) -> Decision {
        let base = self.decide_base(g, who, right);
        if !base.allowed
            && base.reason != Reason::OperatorGroup
            && self.people.status(who) == Some(false)
            && let Some(i) = self.open_iv(g, who)
        {
            return Decision {
                allowed: true,
                reason: Reason::Holder { grant: i.id },
            };
        }
        base
    }

    /// `decide` without an intervention: rights of her own (grants and the owner/manager roles; the installer has no standing right: she only hands out the first `Project` scope).
    pub(crate) fn decide_base(&self, g: ObjectId, who: &PersonRef, right: Right) -> Decision {
        let no = |reason| Decision {
            allowed: false,
            reason,
        };
        if right != Right::Read && self.is_operator_group(g) {
            // The group "Betreiber" is written by the local command only: no grant, no role, no intervention.
            return no(Reason::OperatorGroup);
        }
        let status = self.people.status(who);
        if status == Some(true) {
            let closed =
                self.groups.get(&g).is_some_and(|r| r.group.sensitive) || self.is_admin_group(g);
            if closed || right == Right::Manage {
                return no(Reason::Agent);
            }
        } else if status.is_none() && right == Right::Manage {
            return no(Reason::AiStatusUnknown);
        }
        if let Some(m) = self.role_membership(g, who) {
            return Decision {
                allowed: true,
                reason: Reason::Holder { grant: m.id },
            };
        }
        for grant in self
            .grants
            .iter()
            .filter(|x| x.group == g && covers(x.right, right))
        {
            if self.grant_force(grant) != InForce::Yes {
                continue;
            }
            if &grant.holder == who {
                return Decision {
                    allowed: true,
                    reason: Reason::Holder { grant: grant.id },
                };
            }
            let Some(holder) =
                id_of_reference(&grant.holder.id).filter(|_| grant.holder.art == PersonArt::Group)
            else {
                continue;
            };
            if status != Some(true) && self.holds_member(holder, who) {
                let shown = self.readable(holder).then_some(holder);
                return Decision {
                    allowed: true,
                    reason: Reason::ViaGroup {
                        grant: grant.id,
                        holder: shown,
                    },
                };
            }
        }
        no(match status {
            Some(true) => Reason::Agent,
            None => Reason::AiStatusUnknown,
            Some(false) => Reason::NoGrant,
        })
    }

    /// How many humans hold `Manage` at `g` now (in force, directly or over a holder group).
    pub(crate) fn manage_holders(&self, g: ObjectId) -> usize {
        let mut who: Vec<PersonRef> = Vec::new();
        for grant in self
            .grants
            .iter()
            .filter(|x| x.group == g && x.right == Right::Manage)
        {
            if self.grant_force(grant) != InForce::Yes {
                continue;
            }
            if grant.holder.art == PersonArt::Group {
                let Some(h) = id_of_reference(&grant.holder.id) else {
                    continue;
                };
                who.extend(self.effective_raw(h).into_iter().map(|e| e.person));
            } else {
                who.push(grant.holder.clone());
            }
        }
        who.extend(self.role_holders(g));
        who.retain(|p| self.people.status(p) == Some(false));
        let mut keys: Vec<String> = who.iter().map(key).collect();
        keys.sort();
        keys.dedup();
        keys.len()
    }

    /// Whether taking away one membership or one grant leaves the group without any `Manage` holder (it had one).
    pub(crate) fn would_orphan(
        &mut self,
        g: ObjectId,
        member: Option<ObjectId>,
        grant: Option<ObjectId>,
    ) -> bool {
        let before = self.manage_holders(g);
        let (members, grants) = (self.members.clone(), self.grants.clone());
        self.members.retain(|m| Some(m.id) != member);
        self.grants.retain(|x| Some(x.id) != grant);
        let after = self.manage_holders(g);
        (self.members, self.grants) = (members, grants);
        before > 0 && after == 0
    }

    /// Whether giving membership `member` the role `role` leaves the group without any `Manage` holder (it had one).
    /// A group with an owner never ends up there: the owner role is itself `Manage` (ADR 0095 Pkt. 8).
    pub(crate) fn would_orphan_role(
        &mut self,
        g: ObjectId,
        member: ObjectId,
        role: Option<&RoleRef>,
    ) -> bool {
        let before = self.manage_holders(g);
        let members = self.members.clone();
        if let Some(row) = self.members.iter_mut().find(|m| m.id == member) {
            row.role = role.cloned();
        }
        let after = self.manage_holders(g);
        self.members = members;
        before > 0 && after == 0
    }
}
