//! `ConformanceFixture`: how a provider under test is put into the states the rules need.
//!
//! Pattern: Strategy / Test fixture (the rules are written once; each provider says how to make a person of a kind,
//! who asks and how the clock moves, so the same rules run against the fake and later against `LocalGroups`).

use crate::groups::shared::{ObjectId, PersonRef};
use crate::groups::{GroupDirectory, GroupMembers, Groups, Mirroring, RoleRef};
use crate::groups::{GroupGovernance, LocalOperator};
use async_trait::async_trait;

/// What people say about a person's account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Marked as a human.
    Human,
    /// Marked as an AI account.
    Agent,
    /// Nobody knows.
    Unknown,
}

/// A change made by hand in the directory (Kanidm) behind the provider.
#[derive(Clone, Debug)]
pub enum ByHand {
    /// An account that fs-groups does not know, added to a group.
    Foreign { group: ObjectId, account: String },
    /// `inner` laid into `outer` by hand.
    Nest { outer: ObjectId, inner: ObjectId },
    /// The directory goes down (`true`) or comes back.
    Down(bool),
}

/// The provider under test plus the mutators behind it. A mutator the provider cannot offer keeps its default
/// (`false` / `None`); the rules that need it are skipped.
#[async_trait]
pub trait ConformanceFixture: Send + Sync {
    /// The provider under test.
    fn groups(&self) -> &dyn Groups;

    /// The `GroupGovernance` view of the same provider; `None` skips the governance rules. For the scope rules the asking person at
    /// the start of the run is the installer of the provider (she hands out the first `Project` scope).
    fn governance(&self) -> Option<&dyn GroupGovernance> {
        None
    }

    /// The LOCAL entry of the operator command (`LocalOperator`); `None` skips the operator rules. It is the server
    /// program's way in, never a path of the contract.
    fn local_operator(&self) -> Option<&dyn LocalOperator> {
        None
    }

    /// The `GroupMembers` view of the same provider.
    fn facade(&self) -> Option<&dyn GroupMembers> {
        None
    }

    /// The `Mirroring` view of the same provider.
    fn mirroring(&self) -> Option<&dyn Mirroring> {
        None
    }

    /// The directory the mirror runs against.
    fn directory(&self) -> Option<&dyn GroupDirectory> {
        None
    }

    /// A new person of this kind in the people book.
    async fn person(&self, _kind: Kind) -> Option<PersonRef> {
        None
    }

    /// Change what people say about an existing person (the mark changes after the fact).
    async fn mark(&self, _person: &PersonRef, _kind: Kind) -> bool {
        false
    }

    /// Mark an account as supervised (`Some(true)`), not (`Some(false)`) or not known (`None`, ADR 0095 Pkt. 14).
    /// `true` when done.
    async fn supervise(&self, _person: &PersonRef, _on: Option<bool>) -> bool {
        false
    }

    /// A role that is not an admin role, and one that is.
    async fn roles(&self) -> Option<(RoleRef, RoleRef)> {
        None
    }

    /// Say who asks from now on. `true` when done.
    async fn act_as(&self, _who: &PersonRef) -> bool {
        false
    }

    /// The person who asks at the start of the run (a human).
    async fn me(&self) -> Option<PersonRef> {
        None
    }

    /// Move the provider's clock forward (invitations and the trash run out). `true` when done.
    async fn advance_clock(&self, _millis: i64) -> bool {
        false
    }

    /// Change the deleted group behind the provider's back (what `restore` answers with `Changed`).
    async fn touch_deleted(&self, _group: ObjectId) -> bool {
        false
    }

    /// A change by hand in the directory.
    async fn by_hand(&self, _change: ByHand) -> bool {
        false
    }
}
