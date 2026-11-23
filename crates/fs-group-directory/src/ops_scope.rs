//! Levels (a) and (b) as groups with a scope, and the intervention (ADR 0095 Pkt. 5, 7): `grant_scope`,
//! `revoke_scope`, `scopes`, `open_intervention`, `close_intervention`, `interventions`.
//!
//! Pattern: Command handlers on the state. A scope is held by a GROUP and works for its direct human members; it
//! never works without an intervention (Proxy: the acts are noted in `State::log`, see `v2_state.rs`).

use crate::groups_v1::shared::{ObjectId, PersonRef, Revision, Utc};
use crate::groups_v1::{Error, Missing};
use crate::groups_v2::{Error2, INTERVENTION_MS, Intervention, Result2, Scope, ScopeGrant};
use crate::state::State;

impl State {
    /// The scope grant through which `who` holds `wanted` (she is a direct human member of the admin group).
    fn scope_via(&self, who: &PersonRef, wanted: Scope) -> Option<&ScopeGrant> {
        if self.people.status(who) != Some(false) {
            return None;
        }
        self.v2.scopes.iter().find(|s| {
            s.scope.covers(wanted)
                && self
                    .groups
                    .get(&s.admin_group)
                    .is_some_and(|r| r.deleted.is_none())
                && self
                    .members
                    .iter()
                    .any(|m| m.group == s.admin_group && &m.person == who)
        })
    }

    /// The asker may hand out scopes: she holds `Project`, or nothing is handed out yet and she is the installer.
    fn may_hand_out(&self) -> bool {
        let first = !self.v2.scopes.iter().any(|s| s.scope == Scope::Project);
        self.scope_via(&self.asker, Scope::Project).is_some()
            || (first
                && self.installer.as_ref() == Some(&self.asker)
                && self.people.status(&self.asker) == Some(false))
    }

    pub(crate) fn grant_scope(&mut self, admin: ObjectId, scope: Scope) -> Result2<ScopeGrant> {
        if matches!(scope, Scope::Group(_)) || !self.may_hand_out() {
            return Err(Error2::ScopeDenied(scope));
        }
        self.visible(admin)?;
        if let Some(s) = self
            .v2
            .scopes
            .iter()
            .find(|s| s.admin_group == admin && s.scope == scope)
        {
            return Err(Error::AlreadyThere(s.id).into());
        }
        let now = self.tick();
        let grant = ScopeGrant {
            id: self.fresh(),
            revision: Revision(1),
            admin_group: admin,
            scope,
            granted_by: self.asker.clone(),
            granted: now,
        };
        self.v2.scopes.push(grant.clone());
        Ok(grant)
    }

    pub(crate) fn revoke_scope(&mut self, id: ObjectId) -> Result2<()> {
        let missing = || Error::NotFound(Missing::Grant(id));
        let g = self
            .v2
            .scopes
            .iter()
            .find(|s| s.id == id)
            .cloned()
            .ok_or_else(missing)?;
        if !self.may_hand_out() {
            return Err(Error2::ScopeDenied(g.scope));
        }
        let projects = self
            .v2
            .scopes
            .iter()
            .filter(|s| s.scope == Scope::Project)
            .count();
        if g.scope == Scope::Project && projects <= 1 {
            return Err(Error2::LastScope);
        }
        self.tick();
        self.v2.scopes.retain(|s| s.id != id);
        Ok(())
    }

    pub(crate) fn scopes(&self) -> Result2<Vec<ScopeGrant>> {
        if self.scope_via(&self.asker, Scope::AllGroups).is_none() && !self.may_hand_out() {
            return Err(Error2::ScopeDenied(Scope::AllGroups));
        }
        Ok(self.v2.scopes.clone())
    }

    pub(crate) fn open_intervention(
        &mut self,
        group: ObjectId,
        scope: Scope,
        reason: &str,
    ) -> Result2<Intervention> {
        self.visible(group)?;
        if matches!(scope, Scope::Group(_)) {
            return Err(Error2::ScopeDenied(scope));
        }
        if reason.trim().is_empty() {
            return Err(Error2::ReasonMissing);
        }
        let via = self
            .scope_via(&self.asker, scope)
            .map(|s| s.admin_group)
            .ok_or(Error2::ScopeDenied(scope))?;
        let now = self.tick();
        let iv = Intervention {
            id: self.fresh(),
            group,
            actor: self.asker.clone(),
            admin_group: via,
            scope,
            reason: reason.trim().to_owned(),
            opened: now,
            ends: Utc::from_millis(now.millis() + INTERVENTION_MS),
            closed: None,
            acts: Vec::new(),
        };
        self.v2.interventions.push(iv.clone());
        Ok(iv)
    }

    pub(crate) fn close_intervention(&mut self, id: ObjectId) -> Result2<()> {
        let me = self.asker.clone();
        let now = self.tick();
        let open = self
            .v2
            .interventions
            .iter_mut()
            .find(|i| i.id == id && i.actor == me && i.closed.is_none() && now < i.ends)
            .ok_or(Error2::NoSuchIntervention(id))?;
        open.closed = Some(now);
        Ok(())
    }

    pub(crate) fn interventions(&self, group: ObjectId) -> Result2<Vec<Intervention>> {
        self.v2_gate(group)?;
        Ok(self
            .v2
            .interventions
            .iter()
            .filter(|i| i.group == group)
            .cloned()
            .collect())
    }
}
