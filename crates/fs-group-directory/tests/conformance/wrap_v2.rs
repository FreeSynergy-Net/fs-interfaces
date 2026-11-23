//! A `GroupsV2` provider that forwards to a `FakeGroups` and breaks ONE rule of v2 on purpose (a mutant), to show that
//! the conformance rules of v2 notice it.

use async_trait::async_trait;
use fs_group_directory::FakeGroups;
use fs_group_directory::groups_v1::shared::{ObjectId, PersonRef, Revision, Utc};
use fs_group_directory::groups_v1::{Group, GroupDraft, Groups, InForce, Membership, NestOutcome};
use fs_group_directory::groups_v2::{
    Addon, AddonKind, Error2, GroupsV2, Intervention, Level, Result2, Scope, ScopeGrant,
};
use std::sync::Arc;

/// The rule that is broken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutantV2 {
    /// Nothing is broken.
    None,
    /// `owners` answers an empty list where the group is withheld.
    EmptyForWithheld,
    /// `attach_addon` lets anyone attach (the level is not asked).
    AddonWithoutRight,
    /// `demote_owner` lets the last owner go.
    LastOwnerGoes,
    /// `transfer_ownership` trusts the membership named instead of the connection.
    ActorFromArgument,
    /// `appoint_owner` asks no Owner level (a manager appoints).
    AppointWithoutOwner,
    /// `open_intervention` takes an empty reason.
    NoReason,
    /// `attach_addon` also adds the asker as a member (an addon that changes something).
    AddonChangesMembers,
}

pub struct WrapV2(pub Arc<FakeGroups>, pub MutantV2);

/// What a refused call looks like to the mutants.
fn is(e: &Result2<impl Sized>, f: impl Fn(&Error2) -> bool) -> bool {
    e.as_ref().err().is_some_and(f)
}

#[async_trait]
impl GroupsV2 for WrapV2 {
    async fn create_owned(&self, d: GroupDraft) -> Result2<Group> {
        self.0.create_owned(d).await
    }
    async fn owners(&self, g: ObjectId) -> Result2<Vec<Membership>> {
        let r = self.0.owners(g).await;
        if self.1 == MutantV2::EmptyForWithheld && is(&r, |e| matches!(e, Error2::Withheld { .. }))
        {
            return Ok(Vec::new());
        }
        r
    }
    async fn my_level(&self, g: ObjectId) -> Result2<Level> {
        self.0.my_level(g).await
    }
    async fn appoint_owner(&self, g: ObjectId, p: &PersonRef) -> Result2<Membership> {
        let r = self.0.appoint_owner(g, p).await;
        if self.1 == MutantV2::AppointWithoutOwner
            && is(&r, |e| matches!(e, Error2::NeedsOwner { .. }))
        {
            // a manager (not an owner) gets through: the plain member entry of the candidate, as if appointed
            return self.0.add_member(g, p, None).await.map_err(Error2::V1);
        }
        r
    }
    async fn demote_owner(&self, m: ObjectId, rev: Revision) -> Result2<Membership> {
        let r = self.0.demote_owner(m, rev).await;
        if self.1 == MutantV2::LastOwnerGoes && is(&r, |e| matches!(e, Error2::LastOwner { .. })) {
            return Ok(Membership {
                id: m,
                revision: rev,
                group: m,
                person: self.0.asker(),
                role: None,
                since: Utc::from_millis(0),
                added_by: self.0.asker(),
                from_invitation: None,
                in_force: InForce::Yes,
            });
        }
        r
    }
    async fn transfer_ownership(
        &self,
        f: ObjectId,
        rev: Revision,
        to: &PersonRef,
        keep: bool,
    ) -> Result2<()> {
        let r = self.0.transfer_ownership(f, rev, to, keep).await;
        if self.1 == MutantV2::ActorFromArgument
            && is(&r, |e| matches!(e, Error2::NotYourMembership(_)))
        {
            return Ok(());
        }
        r
    }
    async fn resolve_owner(&self, o: &PersonRef) -> Result2<Vec<PersonRef>> {
        self.0.resolve_owner(o).await
    }
    async fn nest_default(&self, c: ObjectId, p: ObjectId) -> Result2<NestOutcome> {
        self.0.nest_default(c, p).await
    }
    async fn scopes(&self) -> Result2<Vec<ScopeGrant>> {
        self.0.scopes().await
    }
    async fn grant_scope(&self, a: ObjectId, s: Scope) -> Result2<ScopeGrant> {
        self.0.grant_scope(a, s).await
    }
    async fn revoke_scope(&self, g: ObjectId) -> Result2<()> {
        self.0.revoke_scope(g).await
    }
    async fn open_intervention(
        &self,
        g: ObjectId,
        s: Scope,
        reason: &str,
    ) -> Result2<Intervention> {
        if self.1 == MutantV2::NoReason {
            return self.0.open_intervention(g, s, "x").await;
        }
        self.0.open_intervention(g, s, reason).await
    }
    async fn close_intervention(&self, i: ObjectId) -> Result2<()> {
        self.0.close_intervention(i).await
    }
    async fn interventions(&self, g: ObjectId) -> Result2<Vec<Intervention>> {
        self.0.interventions(g).await
    }
    async fn addons(&self, g: ObjectId) -> Result2<Vec<Addon>> {
        self.0.addons(g).await
    }
    async fn attach_addon(&self, g: ObjectId, k: AddonKind) -> Result2<Addon> {
        let r = self.0.attach_addon(g, k.clone()).await;
        if self.1 == MutantV2::AddonWithoutRight
            && is(&r, |e| matches!(e, Error2::NeedsLevel { .. }))
        {
            return Ok(Addon {
                id: g,
                revision: Revision(1),
                group: g,
                kind: k,
                attached_by: self.0.asker(),
                attached: Utc::from_millis(0),
            });
        }
        if self.1 == MutantV2::AddonChangesMembers && r.is_ok() {
            let me = self.0.asker();
            let _ = Groups::add_member(self.0.as_ref(), g, &me, None).await;
        }
        r
    }
    async fn update_addon(&self, a: ObjectId, rev: Revision, k: AddonKind) -> Result2<Addon> {
        self.0.update_addon(a, rev, k).await
    }
    async fn detach_addon(&self, a: ObjectId, rev: Revision) -> Result2<()> {
        self.0.detach_addon(a, rev).await
    }
}
