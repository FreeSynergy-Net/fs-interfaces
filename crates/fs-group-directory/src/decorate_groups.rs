//! `Groups` through the gate: one gate call, then the provider inside.
//!
//! Pattern: Decorator (the `Groups` half of [`Decorated`]).

use crate::decorate::{Decorated, Gate, Op};
use crate::groups_v1::shared::{ObjectId, Page, PersonRef, Place, Reference, Revision, Utc};
use crate::groups_v1::{
    Change, Decision, Deleted, EffectiveGroup, EffectiveMember, Grant, Group, GroupDraft,
    GroupPatch, GroupQuery, GroupSummary, Groups, HistoryEntry, Invitation, InvitationAnswer,
    InvitationOutcome, Membership, NestOutcome, Nesting, NestingEffect, NestingMode,
    NestingRequest, NestingSide, ParentFilter, RequestAnswer, RequestSide, Result, Right, RoleRef,
};
use async_trait::async_trait;

#[async_trait]
impl<G: Gate> Groups for Decorated<G> {
    async fn list(
        &self,
        q: &GroupQuery,
        a: Option<&Place>,
        n: usize,
    ) -> Result<Page<GroupSummary>> {
        let named = match q.parent {
            ParentFilter::ChildrenOf(id) => vec![id],
            _ => Vec::new(),
        };
        self.gate.pass(Op::List, &named)?;
        self.inner.list(q, a, n).await
    }
    async fn get(&self, id: ObjectId) -> Result<Option<Group>> {
        self.gate.pass(Op::Get, &[id])?;
        self.inner.get(id).await
    }
    async fn get_by_reference(&self, r: &Reference) -> Result<Option<Group>> {
        self.gate.pass(Op::GetByReference, &[])?;
        self.inner.get_by_reference(r).await
    }
    async fn create(&self, d: GroupDraft) -> Result<Group> {
        self.gate.pass(Op::Create, &[])?;
        self.inner.create(d).await
    }
    async fn update(&self, id: ObjectId, e: Revision, p: GroupPatch) -> Result<Group> {
        self.gate.pass(Op::Update, &[id])?;
        self.inner.update(id, e, p).await
    }
    async fn delete(&self, id: ObjectId, e: Revision) -> Result<Deleted> {
        self.gate.pass(Op::Delete, &[id])?;
        self.inner.delete(id, e).await
    }
    async fn restore(&self, id: ObjectId) -> Result<Group> {
        self.gate.pass(Op::Restore, &[id])?;
        self.inner.restore(id).await
    }
    async fn request_nesting(
        &self,
        c: ObjectId,
        p: ObjectId,
        m: NestingMode,
    ) -> Result<NestOutcome> {
        self.gate.pass(Op::RequestNesting, &[c, p])?;
        self.inner.request_nesting(c, p, m).await
    }
    async fn nesting_effect(
        &self,
        c: ObjectId,
        p: ObjectId,
        m: NestingMode,
    ) -> Result<NestingEffect> {
        self.gate.pass(Op::NestingEffect, &[c, p])?;
        self.inner.nesting_effect(c, p, m).await
    }
    async fn nestings(&self, g: ObjectId, s: NestingSide) -> Result<Vec<Nesting>> {
        self.gate.pass(Op::Nestings, &[g])?;
        self.inner.nestings(g, s).await
    }
    async fn nesting_requests(&self, g: ObjectId, s: RequestSide) -> Result<Vec<NestingRequest>> {
        self.gate.pass(Op::NestingRequests, &[g])?;
        self.inner.nesting_requests(g, s).await
    }
    async fn answer_nesting_request(
        &self,
        r: ObjectId,
        e: Revision,
        a: RequestAnswer,
    ) -> Result<NestingRequest> {
        self.gate.pass(Op::AnswerNestingRequest, &[])?;
        self.inner.answer_nesting_request(r, e, a).await
    }
    async fn withdraw_nesting_request(&self, r: ObjectId, e: Revision) -> Result<NestingRequest> {
        self.gate.pass(Op::WithdrawNestingRequest, &[])?;
        self.inner.withdraw_nesting_request(r, e).await
    }
    async fn change_nesting_mode(
        &self,
        n: ObjectId,
        e: Revision,
        m: NestingMode,
    ) -> Result<NestOutcome> {
        self.gate.pass(Op::ChangeNestingMode, &[])?;
        self.inner.change_nesting_mode(n, e, m).await
    }
    async fn remove_nesting(&self, n: ObjectId, e: Revision) -> Result<()> {
        self.gate.pass(Op::RemoveNesting, &[])?;
        self.inner.remove_nesting(n, e).await
    }
    async fn ancestors(&self, id: ObjectId) -> Result<Vec<ObjectId>> {
        self.gate.pass(Op::Ancestors, &[id])?;
        self.inner.ancestors(id).await
    }
    async fn descendants(&self, id: ObjectId) -> Result<Vec<ObjectId>> {
        self.gate.pass(Op::Descendants, &[id])?;
        self.inner.descendants(id).await
    }
    async fn members(&self, g: ObjectId, a: Option<&Place>, n: usize) -> Result<Page<Membership>> {
        self.gate.pass(Op::Members, &[g])?;
        self.inner.members(g, a, n).await
    }
    async fn effective_members(
        &self,
        g: ObjectId,
        a: Option<&Place>,
        n: usize,
    ) -> Result<Page<EffectiveMember>> {
        self.gate.pass(Op::EffectiveMembers, &[g])?;
        self.inner.effective_members(g, a, n).await
    }
    async fn add_member(
        &self,
        g: ObjectId,
        p: &PersonRef,
        r: Option<&RoleRef>,
    ) -> Result<Membership> {
        self.gate.pass(Op::AddMember, &[g])?;
        self.inner.add_member(g, p, r).await
    }
    async fn set_role(&self, m: ObjectId, e: Revision, r: Option<&RoleRef>) -> Result<Membership> {
        self.gate.pass(Op::SetRole, &[])?;
        self.inner.set_role(m, e, r).await
    }
    async fn remove_member(&self, m: ObjectId, e: Revision) -> Result<()> {
        self.gate.pass(Op::RemoveMember, &[])?;
        self.inner.remove_member(m, e).await
    }
    async fn groups_of(&self, p: &PersonRef) -> Result<Vec<EffectiveGroup>> {
        self.gate.pass(Op::GroupsOf, &[])?;
        self.inner.groups_of(p).await
    }
    async fn invitations(&self, g: ObjectId) -> Result<Vec<Invitation>> {
        self.gate.pass(Op::Invitations, &[g])?;
        self.inner.invitations(g).await
    }
    async fn my_invitations(&self) -> Result<Vec<Invitation>> {
        self.gate.pass(Op::MyInvitations, &[])?;
        self.inner.my_invitations().await
    }
    async fn invite(
        &self,
        g: ObjectId,
        who: &PersonRef,
        r: Option<&RoleRef>,
        x: Option<Utc>,
    ) -> Result<Invitation> {
        self.gate.pass(Op::Invite, &[g])?;
        self.inner.invite(g, who, r, x).await
    }
    async fn answer_invitation(
        &self,
        i: ObjectId,
        e: Revision,
        a: InvitationAnswer,
    ) -> Result<InvitationOutcome> {
        self.gate.pass(Op::AnswerInvitation, &[])?;
        self.inner.answer_invitation(i, e, a).await
    }
    async fn revoke_invitation(&self, i: ObjectId, e: Revision) -> Result<Invitation> {
        self.gate.pass(Op::RevokeInvitation, &[])?;
        self.inner.revoke_invitation(i, e).await
    }
    async fn grants(&self, g: ObjectId) -> Result<Vec<Grant>> {
        self.gate.pass(Op::Grants, &[g])?;
        self.inner.grants(g).await
    }
    async fn grant(&self, g: ObjectId, h: &PersonRef, r: Right) -> Result<Grant> {
        self.gate.pass(Op::Grant, &[g])?;
        self.inner.grant(g, h, r).await
    }
    async fn revoke_grant(&self, g: ObjectId, e: Revision) -> Result<()> {
        self.gate.pass(Op::RevokeGrant, &[])?;
        self.inner.revoke_grant(g, e).await
    }
    async fn may_do(&self, g: ObjectId, who: &PersonRef, r: Right) -> Result<Decision> {
        self.gate.pass(Op::MayDo, &[g])?;
        self.inner.may_do(g, who, r).await
    }
    async fn my_rights(&self, g: ObjectId) -> Result<Vec<Right>> {
        self.gate.pass(Op::MyRights, &[g])?;
        self.inner.my_rights(g).await
    }
    async fn changes(&self, s: Option<&Place>, n: usize) -> Result<Page<Change>> {
        self.gate.pass(Op::Changes, &[])?;
        self.inner.changes(s, n).await
    }
    async fn history(
        &self,
        g: ObjectId,
        a: Option<&Place>,
        n: usize,
    ) -> Result<Page<HistoryEntry>> {
        self.gate.pass(Op::History, &[g])?;
        self.inner.history(g, a, n).await
    }
}
