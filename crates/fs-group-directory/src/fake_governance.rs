//! The `GroupGovernance` impl of `FakeGroups`: every method takes the lock and calls the state (no logic here).
//!
//! Pattern: Adapter (the async trait on one side, the sync state operations on the other).

use crate::fake::FakeGroups;
use crate::groups::shared::{ObjectId, PersonRef, Revision};
use crate::groups::{
    Addon, AddonKind, GroupGovernance, Intervention, Level, Result, Scope, ScopeGrant,
};
use crate::groups::{Group, GroupDraft, Membership, NestOutcome};
use async_trait::async_trait;

#[async_trait]
impl GroupGovernance for FakeGroups {
    async fn create_owned(&self, draft: GroupDraft) -> Result<Group> {
        self.lock().create_owned(draft)
    }
    async fn owners(&self, group: ObjectId) -> Result<Vec<Membership>> {
        self.lock().owners(group)
    }
    async fn my_level(&self, group: ObjectId) -> Result<Level> {
        self.lock().my_level(group)
    }
    async fn appoint_owner(&self, group: ObjectId, person: &PersonRef) -> Result<Membership> {
        self.lock().appoint_owner(group, person)
    }
    async fn demote_owner(&self, membership: ObjectId, expected: Revision) -> Result<Membership> {
        self.lock().demote_owner(membership, expected)
    }
    async fn transfer_ownership(
        &self,
        from: ObjectId,
        expected: Revision,
        to: &PersonRef,
        keep: bool,
    ) -> Result<()> {
        self.lock().transfer_ownership(from, expected, to, keep)
    }
    async fn resolve_owner(&self, owner: &PersonRef) -> Result<Vec<PersonRef>> {
        self.lock().resolve_owner(owner)
    }
    async fn nest_default(&self, child: ObjectId, parent: ObjectId) -> Result<NestOutcome> {
        self.lock().nest_default(child, parent)
    }
    async fn scopes(&self) -> Result<Vec<ScopeGrant>> {
        self.lock().scopes()
    }
    async fn grant_scope(&self, admin_group: ObjectId, scope: Scope) -> Result<ScopeGrant> {
        self.lock().grant_scope(admin_group, scope)
    }
    async fn revoke_scope(&self, grant: ObjectId) -> Result<()> {
        self.lock().revoke_scope(grant)
    }
    async fn open_intervention(
        &self,
        group: ObjectId,
        scope: Scope,
        reason: &str,
    ) -> Result<Intervention> {
        self.lock().open_intervention(group, scope, reason)
    }
    async fn close_intervention(&self, intervention: ObjectId) -> Result<()> {
        self.lock().close_intervention(intervention)
    }
    async fn interventions(&self, group: ObjectId) -> Result<Vec<Intervention>> {
        self.lock().interventions(group)
    }
    async fn addons(&self, group: ObjectId) -> Result<Vec<Addon>> {
        self.lock().addons(group)
    }
    async fn attach_addon(&self, group: ObjectId, kind: AddonKind) -> Result<Addon> {
        self.lock().attach_addon(group, kind)
    }
    async fn update_addon(
        &self,
        addon: ObjectId,
        expected: Revision,
        kind: AddonKind,
    ) -> Result<Addon> {
        self.lock().update_addon(addon, expected, kind)
    }
    async fn detach_addon(&self, addon: ObjectId, expected: Revision) -> Result<()> {
        self.lock().detach_addon(addon, expected)
    }
}
