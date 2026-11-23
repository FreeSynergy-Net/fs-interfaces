//! A provider that forwards everything to a `FakeGroups` and breaks ONE rule on purpose (a mutant), to show that
//! the conformance rules notice it.

use async_trait::async_trait;
use fs_group_directory::FakeGroups;
use fs_group_directory::groups_v1::shared::{
    ObjectId, Page, PersonRef, Place, Reference, Revision, Utc,
};
use fs_group_directory::groups_v1::{
    Change, Decision, Deleted, EffectiveGroup, EffectiveMember, Error, Grant, Group, GroupDraft,
    GroupPatch, GroupQuery, GroupSummary, Groups, HistoryEntry, Invitation, InvitationAnswer,
    InvitationOutcome, Membership, NestOutcome, Nesting, NestingEffect, NestingMode,
    NestingRequest, NestingSide, RequestAnswer, RequestSide, Result, Right, RoleRef,
};
use std::sync::Arc;

/// The rule that is broken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mutant {
    /// Nothing is broken: a plain forward.
    None,
    /// `members` of a hidden group answers `Forbidden` instead of `NotFound` (tells that the group exists).
    MembersLeak,
    /// `nesting_effect` answers `Forbidden` where it must answer `NotFound` (the review finding B1).
    EffectLeak,
    /// `request_nesting` calls a cycle `AlreadyThere`.
    CycleAsAlready,
    /// `update` does not look at the revision.
    NoCompareAndSet,
}

/// The forwarding provider.
pub struct Wrap(pub Arc<FakeGroups>, pub Mutant);

fn forbid<T>(r: Result<T>, g: ObjectId) -> Result<T> {
    r.map_err(|e| {
        if matches!(e, Error::NotFound(_)) {
            Error::Forbidden {
                group: g,
                right: Right::Read,
            }
        } else {
            e
        }
    })
}

#[async_trait]
impl Groups for Wrap {
    async fn list(
        &self,
        q: &GroupQuery,
        a: Option<&Place>,
        n: usize,
    ) -> Result<Page<GroupSummary>> {
        self.0.list(q, a, n).await
    }
    async fn get(&self, id: ObjectId) -> Result<Option<Group>> {
        self.0.get(id).await
    }
    async fn get_by_reference(&self, r: &Reference) -> Result<Option<Group>> {
        self.0.get_by_reference(r).await
    }
    async fn create(&self, d: GroupDraft) -> Result<Group> {
        self.0.create(d).await
    }
    async fn update(&self, id: ObjectId, expected: Revision, p: GroupPatch) -> Result<Group> {
        if self.1 == Mutant::NoCompareAndSet {
            let current = self.0.get(id).await?.map_or(expected, |g| g.revision);
            return self.0.update(id, current, p).await;
        }
        self.0.update(id, expected, p).await
    }
    async fn delete(&self, id: ObjectId, e: Revision) -> Result<Deleted> {
        self.0.delete(id, e).await
    }
    async fn restore(&self, id: ObjectId) -> Result<Group> {
        self.0.restore(id).await
    }
    async fn request_nesting(
        &self,
        c: ObjectId,
        p: ObjectId,
        m: NestingMode,
    ) -> Result<NestOutcome> {
        let r = self.0.request_nesting(c, p, m).await;
        if self.1 == Mutant::CycleAsAlready {
            return r.map_err(|e| {
                if matches!(e, Error::Cycle { .. }) {
                    Error::AlreadyThere(c)
                } else {
                    e
                }
            });
        }
        r
    }
    async fn nesting_effect(
        &self,
        c: ObjectId,
        p: ObjectId,
        m: NestingMode,
    ) -> Result<NestingEffect> {
        let r = self.0.nesting_effect(c, p, m).await;
        if self.1 == Mutant::EffectLeak {
            forbid(r, c)
        } else {
            r
        }
    }
    async fn nestings(&self, g: ObjectId, s: NestingSide) -> Result<Vec<Nesting>> {
        self.0.nestings(g, s).await
    }
    async fn nesting_requests(&self, g: ObjectId, s: RequestSide) -> Result<Vec<NestingRequest>> {
        self.0.nesting_requests(g, s).await
    }
    async fn answer_nesting_request(
        &self,
        r: ObjectId,
        e: Revision,
        a: RequestAnswer,
    ) -> Result<NestingRequest> {
        self.0.answer_nesting_request(r, e, a).await
    }
    async fn withdraw_nesting_request(&self, r: ObjectId, e: Revision) -> Result<NestingRequest> {
        self.0.withdraw_nesting_request(r, e).await
    }
    async fn change_nesting_mode(
        &self,
        n: ObjectId,
        e: Revision,
        m: NestingMode,
    ) -> Result<NestOutcome> {
        self.0.change_nesting_mode(n, e, m).await
    }
    async fn remove_nesting(&self, n: ObjectId, e: Revision) -> Result<()> {
        self.0.remove_nesting(n, e).await
    }
    async fn ancestors(&self, id: ObjectId) -> Result<Vec<ObjectId>> {
        self.0.ancestors(id).await
    }
    async fn descendants(&self, id: ObjectId) -> Result<Vec<ObjectId>> {
        self.0.descendants(id).await
    }
    async fn members(&self, g: ObjectId, a: Option<&Place>, n: usize) -> Result<Page<Membership>> {
        let r = self.0.members(g, a, n).await;
        if self.1 == Mutant::MembersLeak {
            forbid(r, g)
        } else {
            r
        }
    }
    async fn effective_members(
        &self,
        g: ObjectId,
        a: Option<&Place>,
        n: usize,
    ) -> Result<Page<EffectiveMember>> {
        self.0.effective_members(g, a, n).await
    }
    async fn add_member(
        &self,
        g: ObjectId,
        p: &PersonRef,
        r: Option<&RoleRef>,
    ) -> Result<Membership> {
        self.0.add_member(g, p, r).await
    }
    async fn set_role(&self, m: ObjectId, e: Revision, r: Option<&RoleRef>) -> Result<Membership> {
        self.0.set_role(m, e, r).await
    }
    async fn remove_member(&self, m: ObjectId, e: Revision) -> Result<()> {
        self.0.remove_member(m, e).await
    }
    async fn groups_of(&self, p: &PersonRef) -> Result<Vec<EffectiveGroup>> {
        self.0.groups_of(p).await
    }
    async fn invitations(&self, g: ObjectId) -> Result<Vec<Invitation>> {
        self.0.invitations(g).await
    }
    async fn my_invitations(&self) -> Result<Vec<Invitation>> {
        self.0.my_invitations().await
    }
    async fn invite(
        &self,
        g: ObjectId,
        i: &PersonRef,
        r: Option<&RoleRef>,
        x: Option<Utc>,
    ) -> Result<Invitation> {
        self.0.invite(g, i, r, x).await
    }
    async fn answer_invitation(
        &self,
        i: ObjectId,
        e: Revision,
        a: InvitationAnswer,
    ) -> Result<InvitationOutcome> {
        self.0.answer_invitation(i, e, a).await
    }
    async fn revoke_invitation(&self, i: ObjectId, e: Revision) -> Result<Invitation> {
        self.0.revoke_invitation(i, e).await
    }
    async fn grants(&self, g: ObjectId) -> Result<Vec<Grant>> {
        self.0.grants(g).await
    }
    async fn grant(&self, g: ObjectId, h: &PersonRef, r: Right) -> Result<Grant> {
        self.0.grant(g, h, r).await
    }
    async fn revoke_grant(&self, g: ObjectId, e: Revision) -> Result<()> {
        self.0.revoke_grant(g, e).await
    }
    async fn may_do(&self, g: ObjectId, w: &PersonRef, r: Right) -> Result<Decision> {
        self.0.may_do(g, w, r).await
    }
    async fn my_rights(&self, g: ObjectId) -> Result<Vec<Right>> {
        self.0.my_rights(g).await
    }
    async fn changes(&self, s: Option<&Place>, n: usize) -> Result<Page<Change>> {
        self.0.changes(s, n).await
    }
    async fn history(
        &self,
        g: ObjectId,
        a: Option<&Place>,
        n: usize,
    ) -> Result<Page<HistoryEntry>> {
        self.0.history(g, a, n).await
    }
}
