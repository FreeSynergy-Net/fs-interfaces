//! `Mirroring` and `GroupMembers` through the gate. A gate error of `Mirroring` that returns `MirrorError` is wrapped
//! as `MirrorError::Groups`.
//!
//! Pattern: Decorator (the other two traits of [`Decorated`]).

use crate::decorate::{Decorated, Gate, Op, group_of};
use crate::groups_v1::shared::{ObjectId, PersonRef};
use crate::groups_v1::{
    DirectoryId, FindingId, GroupCard, GroupDirectory, GroupMembers, GroupMirrorState, MirrorError,
    MirrorPlan, MirrorReport, MirrorStatus, MirrorTicket, Mirroring, Result,
};
use async_trait::async_trait;

#[async_trait]
impl<G: Gate> Mirroring for Decorated<G> {
    async fn mirror_plan(
        &self,
        directory: &dyn GroupDirectory,
    ) -> std::result::Result<MirrorPlan, MirrorError> {
        self.gate
            .pass(Op::MirrorPlan, &[])
            .map_err(MirrorError::Groups)?;
        self.inner.mirror_plan(directory).await
    }
    async fn mirror_apply(
        &self,
        directory: &dyn GroupDirectory,
        ticket: MirrorTicket,
        overwrite: &[FindingId],
    ) -> std::result::Result<MirrorReport, MirrorError> {
        self.gate
            .pass(Op::MirrorApply, &[])
            .map_err(MirrorError::Groups)?;
        self.inner.mirror_apply(directory, ticket, overwrite).await
    }
    async fn mirror_status(&self) -> Result<Vec<MirrorStatus>> {
        self.gate.pass(Op::MirrorStatus, &[])?;
        self.inner.mirror_status().await
    }
    async fn group_mirror_state(
        &self,
        group: ObjectId,
    ) -> Result<Vec<(DirectoryId, GroupMirrorState)>> {
        self.gate.pass(Op::GroupMirrorState, &[group])?;
        self.inner.group_mirror_state(group).await
    }
}

#[async_trait]
impl<G: Gate> GroupMembers for Decorated<G> {
    async fn describe(&self, group: &PersonRef) -> Result<Option<GroupCard>> {
        self.gate.pass(Op::Describe, &group_of(group))?;
        self.inner.describe(group).await
    }
    async fn members_of(&self, group: &PersonRef) -> Result<Vec<PersonRef>> {
        self.gate.pass(Op::MembersOf, &group_of(group))?;
        self.inner.members_of(group).await
    }
    async fn memberships_of(&self, person: &PersonRef) -> Result<Vec<PersonRef>> {
        self.gate.pass(Op::MembershipsOf, &[])?;
        self.inner.memberships_of(person).await
    }
}
