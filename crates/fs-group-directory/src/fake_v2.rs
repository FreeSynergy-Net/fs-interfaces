//! The `GroupsV2` impl of `FakeGroups`: every method takes the lock and calls the state (no logic here).
//!
//! Pattern: Adapter (the async trait on one side, the sync state operations on the other).

use crate::fake::FakeGroups;
use crate::groups_v1::shared::{ObjectId, PersonRef, Revision};
use crate::groups_v1::{Group, GroupDraft, Membership, NestOutcome};
use crate::groups_v2::{
    Addon, AddonKind, GroupsV2, Intervention, Level, Result2, Scope, ScopeGrant,
};
use async_trait::async_trait;

#[async_trait]
impl GroupsV2 for FakeGroups {
    async fn create_owned(&self, draft: GroupDraft) -> Result2<Group> {
        self.lock().create_owned(draft)
    }
    async fn owners(&self, group: ObjectId) -> Result2<Vec<Membership>> {
        self.lock().owners(group)
    }
    async fn my_level(&self, group: ObjectId) -> Result2<Level> {
        self.lock().my_level(group)
    }
    async fn appoint_owner(&self, group: ObjectId, person: &PersonRef) -> Result2<Membership> {
        self.lock().appoint_owner(group, person)
    }
    async fn demote_owner(&self, membership: ObjectId, expected: Revision) -> Result2<Membership> {
        self.lock().demote_owner(membership, expected)
    }
    async fn transfer_ownership(
        &self,
        from: ObjectId,
        expected: Revision,
        to: &PersonRef,
        keep: bool,
    ) -> Result2<()> {
        self.lock().transfer_ownership(from, expected, to, keep)
    }
    async fn resolve_owner(&self, owner: &PersonRef) -> Result2<Vec<PersonRef>> {
        self.lock().resolve_owner(owner)
    }
    async fn nest_default(&self, child: ObjectId, parent: ObjectId) -> Result2<NestOutcome> {
        self.lock().nest_default(child, parent)
    }
    async fn scopes(&self) -> Result2<Vec<ScopeGrant>> {
        self.lock().scopes()
    }
    async fn grant_scope(&self, admin_group: ObjectId, scope: Scope) -> Result2<ScopeGrant> {
        self.lock().grant_scope(admin_group, scope)
    }
    async fn revoke_scope(&self, grant: ObjectId) -> Result2<()> {
        self.lock().revoke_scope(grant)
    }
    async fn open_intervention(
        &self,
        group: ObjectId,
        scope: Scope,
        reason: &str,
    ) -> Result2<Intervention> {
        self.lock().open_intervention(group, scope, reason)
    }
    async fn close_intervention(&self, intervention: ObjectId) -> Result2<()> {
        self.lock().close_intervention(intervention)
    }
    async fn interventions(&self, group: ObjectId) -> Result2<Vec<Intervention>> {
        self.lock().interventions(group)
    }
    async fn addons(&self, group: ObjectId) -> Result2<Vec<Addon>> {
        self.lock().addons(group)
    }
    async fn attach_addon(&self, group: ObjectId, kind: AddonKind) -> Result2<Addon> {
        self.lock().attach_addon(group, kind)
    }
    async fn update_addon(
        &self,
        addon: ObjectId,
        expected: Revision,
        kind: AddonKind,
    ) -> Result2<Addon> {
        self.lock().update_addon(addon, expected, kind)
    }
    async fn detach_addon(&self, addon: ObjectId, expected: Revision) -> Result2<()> {
        self.lock().detach_addon(addon, expected)
    }
}
