//! The decorator both doubles share: every method of `Groups`, `Mirroring` and `GroupMembers` goes through one gate,
//! then to the provider inside.
//!
//! Pattern: Decorator (over the three traits at once) with a Strategy for the gate (`FailGate`, `SlowGate`).

use crate::groups::shared::{ObjectId, PersonArt, PersonRef};
use crate::groups::{GroupMembers, Groups, Mirroring, Result};
use crate::ids::id_of_reference;
use std::sync::Arc;

/// The methods of the three traits a double can fail or slow down.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Op {
    List,
    Get,
    GetByReference,
    Create,
    Update,
    Delete,
    Restore,
    RequestNesting,
    NestingEffect,
    Nestings,
    NestingRequests,
    AnswerNestingRequest,
    WithdrawNestingRequest,
    ChangeNestingMode,
    RemoveNesting,
    Ancestors,
    Descendants,
    Members,
    EffectiveMembers,
    AddMember,
    SetRole,
    RemoveMember,
    GroupsOf,
    Invitations,
    MyInvitations,
    Invite,
    AnswerInvitation,
    RevokeInvitation,
    Grants,
    Grant,
    RevokeGrant,
    MayDo,
    MyRights,
    Changes,
    History,
    MirrorPlan,
    MirrorApply,
    MirrorStatus,
    GroupMirrorState,
    Describe,
    MembersOf,
    MembershipsOf,
}

/// What a gate decides before a call: let it pass (maybe after a virtual wait) or answer with an error. `groups`
/// are the groups the call names by id (empty when it names none, or only a membership, an edge, a request ...).
pub trait Gate: Send + Sync {
    /// # Errors
    /// The error the call answers with instead of reaching the provider inside.
    fn pass(&self, op: Op, groups: &[ObjectId]) -> Result<()>;
}

/// A provider all three traits of the contract come from (the fake, later `LocalGroups`).
pub trait Provider: Groups + Mirroring + GroupMembers {}
impl<T: Groups + Mirroring + GroupMembers> Provider for T {}

/// A provider with a gate in front.
pub struct Decorated<G: Gate> {
    pub(crate) inner: Arc<dyn Provider>,
    pub(crate) gate: G,
}

impl<G: Gate> std::fmt::Debug for Decorated<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Decorated(..)")
    }
}

/// The group a reference to a group or organisation names (nothing for a person or an unreadable reference).
pub(crate) fn group_of(who: &PersonRef) -> Vec<ObjectId> {
    if matches!(who.art, PersonArt::Group | PersonArt::Organization) {
        id_of_reference(&who.id).into_iter().collect()
    } else {
        Vec::new()
    }
}
