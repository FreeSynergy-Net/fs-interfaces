//! The trait impls of `FakeGroups`: every method takes the lock and calls the state (no logic here).
//!
//! Pattern: Adapter (the async trait on one side, the sync state operations on the other).

use crate::fake::FakeGroups;
use crate::groups_v1::shared::{ObjectId, Page, PersonRef, Place, Reference, Revision, Utc};
use crate::groups_v1::{
    Change, Decision, Deleted, DirectoryId, EffectiveGroup, EffectiveMember, FindingId, Grant,
    Group, GroupCard, GroupDirectory, GroupDraft, GroupMembers, GroupMirrorState, GroupPatch,
    GroupQuery, GroupSummary, Groups, HistoryEntry, Invitation, InvitationAnswer,
    InvitationOutcome, Membership, MirrorError, MirrorPlan, MirrorReport, MirrorStatus,
    MirrorTicket, Mirroring, NestOutcome, Nesting, NestingEffect, NestingMode, NestingRequest,
    NestingSide, RequestAnswer, RequestSide, Result, Right, RoleRef,
};
use crate::state::State;
use async_trait::async_trait;

#[async_trait]
impl Groups for FakeGroups {
    async fn list(
        &self,
        query: &GroupQuery,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<GroupSummary>> {
        self.lock().list(query, after, size)
    }
    async fn get(&self, id: ObjectId) -> Result<Option<Group>> {
        Ok(self.lock().get(id))
    }
    async fn get_by_reference(&self, reference: &Reference) -> Result<Option<Group>> {
        Ok(self.lock().get_by_reference(reference))
    }
    async fn create(&self, draft: GroupDraft) -> Result<Group> {
        self.lock().create(draft)
    }
    async fn update(&self, id: ObjectId, expected: Revision, patch: GroupPatch) -> Result<Group> {
        self.lock().update(id, expected, patch)
    }
    async fn delete(&self, id: ObjectId, expected: Revision) -> Result<Deleted> {
        self.lock().delete(id, expected)
    }
    async fn restore(&self, id: ObjectId) -> Result<Group> {
        self.lock().restore(id)
    }
    async fn request_nesting(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Result<NestOutcome> {
        self.lock().request_nesting(child, parent, mode)
    }
    async fn nesting_effect(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Result<NestingEffect> {
        self.lock().nesting_effect(child, parent, mode)
    }
    async fn nestings(&self, group: ObjectId, side: NestingSide) -> Result<Vec<Nesting>> {
        self.lock().nestings(group, side)
    }
    async fn nesting_requests(
        &self,
        group: ObjectId,
        side: RequestSide,
    ) -> Result<Vec<NestingRequest>> {
        self.lock().nesting_requests(group, side)
    }
    async fn answer_nesting_request(
        &self,
        request: ObjectId,
        expected: Revision,
        answer: RequestAnswer,
    ) -> Result<NestingRequest> {
        self.lock()
            .answer_nesting_request(request, expected, answer)
    }
    async fn withdraw_nesting_request(
        &self,
        request: ObjectId,
        expected: Revision,
    ) -> Result<NestingRequest> {
        self.lock().withdraw_nesting_request(request, expected)
    }
    async fn change_nesting_mode(
        &self,
        nesting: ObjectId,
        expected: Revision,
        mode: NestingMode,
    ) -> Result<NestOutcome> {
        self.lock().change_nesting_mode(nesting, expected, mode)
    }
    async fn remove_nesting(&self, nesting: ObjectId, expected: Revision) -> Result<()> {
        self.lock().remove_nesting(nesting, expected)
    }
    async fn ancestors(&self, id: ObjectId) -> Result<Vec<ObjectId>> {
        self.lock().ancestors(id)
    }
    async fn descendants(&self, id: ObjectId) -> Result<Vec<ObjectId>> {
        self.lock().descendants(id)
    }
    async fn members(
        &self,
        group: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<Membership>> {
        self.lock().members(group, after, size)
    }
    async fn effective_members(
        &self,
        group: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<EffectiveMember>> {
        self.lock().effective_members(group, after, size)
    }
    async fn add_member(
        &self,
        group: ObjectId,
        person: &PersonRef,
        role: Option<&RoleRef>,
    ) -> Result<Membership> {
        self.lock().add_member(group, person, role)
    }
    async fn set_role(
        &self,
        membership: ObjectId,
        expected: Revision,
        role: Option<&RoleRef>,
    ) -> Result<Membership> {
        self.lock().set_role(membership, expected, role)
    }
    async fn remove_member(&self, membership: ObjectId, expected: Revision) -> Result<()> {
        self.lock().remove_member(membership, expected)
    }
    async fn groups_of(&self, person: &PersonRef) -> Result<Vec<EffectiveGroup>> {
        self.lock().groups_of(person)
    }
    async fn invitations(&self, group: ObjectId) -> Result<Vec<Invitation>> {
        self.lock().invitations(group)
    }
    async fn my_invitations(&self) -> Result<Vec<Invitation>> {
        Ok(self.lock().my_invitations())
    }
    async fn invite(
        &self,
        group: ObjectId,
        invitee: &PersonRef,
        role: Option<&RoleRef>,
        expires: Option<Utc>,
    ) -> Result<Invitation> {
        self.lock().invite(group, invitee, role, expires)
    }
    async fn answer_invitation(
        &self,
        invitation: ObjectId,
        expected: Revision,
        answer: InvitationAnswer,
    ) -> Result<InvitationOutcome> {
        self.lock().answer_invitation(invitation, expected, answer)
    }
    async fn revoke_invitation(
        &self,
        invitation: ObjectId,
        expected: Revision,
    ) -> Result<Invitation> {
        self.lock().revoke_invitation(invitation, expected)
    }
    async fn grants(&self, group: ObjectId) -> Result<Vec<Grant>> {
        self.lock().grants(group)
    }
    async fn grant(&self, group: ObjectId, holder: &PersonRef, right: Right) -> Result<Grant> {
        self.lock().grant(group, holder, right)
    }
    async fn revoke_grant(&self, grant: ObjectId, expected: Revision) -> Result<()> {
        self.lock().revoke_grant(grant, expected)
    }
    async fn may_do(&self, group: ObjectId, who: &PersonRef, right: Right) -> Result<Decision> {
        self.lock().may_do(group, who, right)
    }
    async fn my_rights(&self, group: ObjectId) -> Result<Vec<Right>> {
        self.lock().my_rights(group)
    }
    async fn changes(&self, since: Option<&Place>, size: usize) -> Result<Page<Change>> {
        Ok(self.lock().changes(since, size))
    }
    async fn history(
        &self,
        group: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<HistoryEntry>> {
        self.lock().history(group, after, size)
    }
}

#[async_trait]
impl GroupMembers for FakeGroups {
    async fn describe(&self, group: &PersonRef) -> Result<Option<GroupCard>> {
        Ok(self.lock().card(group))
    }
    async fn members_of(&self, group: &PersonRef) -> Result<Vec<PersonRef>> {
        self.lock().members_of(group)
    }
    async fn memberships_of(&self, person: &PersonRef) -> Result<Vec<PersonRef>> {
        self.lock().memberships_of(person)
    }
}

type Mirrored<T> = std::result::Result<T, MirrorError>;

#[async_trait]
impl Mirroring for FakeGroups {
    async fn mirror_plan(&self, directory: &dyn GroupDirectory) -> Mirrored<MirrorPlan> {
        self.lock().mirror_gate().map_err(MirrorError::Groups)?;
        let current = directory.list().await.map_err(MirrorError::Directory)?;
        self.lock().mirror_plan(directory.directory_id(), &current)
    }
    async fn mirror_apply(
        &self,
        directory: &dyn GroupDirectory,
        ticket: MirrorTicket,
        overwrite: &[FindingId],
    ) -> Mirrored<MirrorReport> {
        self.lock().mirror_gate().map_err(MirrorError::Groups)?;
        let current = directory.list().await.map_err(MirrorError::Directory)?;
        let plan =
            self.lock()
                .mirror_check(&directory.directory_id(), &ticket, overwrite, &current)?;
        State::mirror_run(&self.state, directory, plan, overwrite).await
    }
    async fn mirror_status(&self) -> Result<Vec<MirrorStatus>> {
        self.lock().mirror_status()
    }
    async fn group_mirror_state(
        &self,
        group: ObjectId,
    ) -> Result<Vec<(DirectoryId, GroupMirrorState)>> {
        self.lock().group_mirror_state(group)
    }
}
