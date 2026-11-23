//! Rights: `grants`, `grant`, `revoke_grant`, `may_do`, `my_rights`.
//!
//! Pattern: Command handlers and queries (the AI rules guard `grant`; the answer of `may_do` names its reason).

use crate::groups_v1::shared::{ObjectId, PersonArt, PersonRef, Revision};
use crate::groups_v1::{
    ChangeKind, Decision, Error, Field, Grant, HistoryKind, InForce, Missing, Result, Right,
};
use crate::ids::id_of_reference;
use crate::state::State;

/// The four rights in order.
const RIGHTS: [Right; 4] = [
    Right::Read,
    Right::AddMembers,
    Right::RemoveMembers,
    Right::Manage,
];

impl State {
    /// A grant as the asker reads it (the force is computed now).
    fn grant_view(&self, g: &Grant) -> Grant {
        Grant {
            in_force: self.grant_force(g),
            ..g.clone()
        }
    }

    pub(crate) fn grants(&self, group: ObjectId) -> Result<Vec<Grant>> {
        self.visible(group)?;
        self.need(group, Right::Read)?;
        let hidden_holder = |g: &Grant| {
            g.holder.art == PersonArt::Group
                && id_of_reference(&g.holder.id).is_some_and(|h| self.hidden(h))
        };
        let mut out: Vec<Grant> = self
            .grants
            .iter()
            .filter(|g| g.group == group && !hidden_holder(g))
            .map(|g| self.grant_view(g))
            .collect();
        out.sort_by_key(|g| (g.granted, g.id));
        Ok(out)
    }

    pub(crate) fn grant(
        &mut self,
        group: ObjectId,
        holder: &PersonRef,
        right: Right,
    ) -> Result<Grant> {
        self.visible(group)?;
        self.need(group, Right::Manage)?;
        let closed = self.alive(group)?.sensitive || self.is_admin_group(group);
        let denied = closed || right == Right::Manage;
        match holder.art {
            PersonArt::Organization => {
                return Err(Error::WrongArt {
                    field: Field::Holder,
                    got: holder.art,
                });
            }
            PersonArt::Group => {
                let missing = || Error::NotFound(Missing::Person(holder.id.clone()));
                let h = id_of_reference(&holder.id).ok_or_else(missing)?;
                self.visible(h).map_err(|_| missing())?;
            }
            PersonArt::Person | PersonArt::Profile => {
                if !self.people.knows(holder) {
                    return Err(Error::NotFound(Missing::Person(holder.id.clone())));
                }
                match self.people.status(holder) {
                    Some(true) if denied => return Err(Error::AgentNotAdmin),
                    None if right == Right::Manage => return Err(Error::AiStatusUnknown),
                    _ => {}
                }
            }
        }
        if let Some(g) = self
            .grants
            .iter()
            .find(|g| g.group == group && &g.holder == holder && g.right == right)
        {
            return Err(Error::AlreadyThere(g.id));
        }
        let now = self.tick();
        let grant = Grant {
            id: self.fresh(),
            revision: Revision(1),
            group,
            holder: holder.clone(),
            right,
            granted_by: self.asker.clone(),
            granted: now,
            in_force: InForce::Yes,
        };
        self.grants.push(grant.clone());
        self.log(
            group,
            HistoryKind::Granted {
                holder: holder.clone(),
                right,
            },
        );
        self.change(ChangeKind::Grant { group });
        Ok(self.grant_view(&grant))
    }

    pub(crate) fn revoke_grant(&mut self, id: ObjectId, expected: Revision) -> Result<()> {
        let missing = || Error::NotFound(Missing::Grant(id));
        let g = self
            .grants
            .iter()
            .find(|g| g.id == id)
            .cloned()
            .ok_or_else(missing)?;
        self.visible(g.group).map_err(|_| missing())?;
        self.need(g.group, Right::Manage)?;
        Self::cas(g.revision, expected)?;
        if self.would_orphan(g.group, None, Some(id)) {
            return Err(Error::LastManager);
        }
        self.tick();
        self.grants.retain(|x| x.id != id);
        self.log(
            g.group,
            HistoryKind::GrantRevoked {
                holder: g.holder,
                right: g.right,
            },
        );
        self.change(ChangeKind::Grant { group: g.group });
        Ok(())
    }

    pub(crate) fn may_do(
        &self,
        group: ObjectId,
        who: &PersonRef,
        right: Right,
    ) -> Result<Decision> {
        self.visible(group)?;
        self.check_person(who)?;
        Ok(self.decide(group, who, right))
    }

    pub(crate) fn my_rights(&self, group: ObjectId) -> Result<Vec<Right>> {
        self.visible(group)?;
        Ok(RIGHTS
            .into_iter()
            .filter(|r| self.may_ask(group, *r))
            .collect())
    }
}
