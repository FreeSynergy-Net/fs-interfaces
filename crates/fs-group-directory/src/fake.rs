//! `FakeGroups`: the in-memory provider every other part is tested against first (plan step K0).
//!
//! Pattern: Fake (a test double with working behaviour) behind the `Groups` Repository trait. It answers every
//! method of the locked trait with the rules of the trait docs and the plan. [`DECISIONS`] lists what it decided
//! where the trait is silent.

use crate::groups_v1::shared::{ObjectId, PersonRef, Utc};
use crate::people::FakePeople;
use crate::state::State;
use std::sync::{Arc, Mutex, MutexGuard};

/// "Now" of a new fake: 2026-11-16T09:00:00Z (the sample's today).
pub const START_MS: i64 = 1_794_819_600_000;

/// What the fake decided where the locked trait is silent (for the conformance rules and the DESIGN).
pub const DECISIONS: [&str; 15] = [
    "Every write takes one second on the fake's own clock (set_now moves it); the trash deadline is 30 days.",
    "Order of checks for a call with a group id: hidden or unknown or deleted (NotFound), then Forbidden, then Changed, then the input (Invalid), then the rule errors.",
    "Rights: Manage covers every right; every right covers Read; AddMembers and RemoveMembers are separate. The installer (if set) may everything and is never counted for LastManager.",
    "A group is an admin group when a living membership carries an admin role of the role book; it is hidden from an asking AI like a sensitive one.",
    "A person is an AI when people say so (Some(true)); a person people do not mark counts as unknown (None) and never gets Manage or an admin role; a person people do not know at all is NotFound(Person).",
    "InForce is computed when read: KindMismatch when the kind of account does not fit the group (an AI in a human group, a human or unknown in an AI group, an AI with an admin role or Manage), AiStatusUnknown for an unknown with an admin role or Manage.",
    "restore needs Manage (judged on the grants the deletion took) and says Changed when the deleted group's revision moved (the fixture mutator touch_deleted); edges that no longer fit stay away and are logged.",
    "An invitation past its date reads as Expired in lists; answering one stores Expired and says Expired { at }; a second answer says InvitationClosed(Expired).",
    "LastManager: removing a membership or a grant that would leave a group with no human holding Manage.",
    "Changes: the cursor is the number last seen; since = None gives an empty page whose next is 'now'; every page has a next.",
    "Mirror: the plan lists a step per group that differs; a nesting by hand that fs-groups never wrote is a ForeignNesting finding (overwritable), one it wrote earlier is an unnest step; a Remove step is planned for every directory group fs-groups no longer wants.",
    "Mirror and asking AI: every read and write of Mirroring is Forbidden with the nil id (the AI may not know any group's id).",
    "A self edge at an AI group is Cycle [A, A] (the cycle check comes before the AI rule); group-nesting says AiNesting there. Known deviation, decided at the switch (DESIGN.md).",
    "Old edges at an AI group count like Single everywhere (hull, cycle check, depth check); group-nesting counts them in the cycle and depth checks. Known deviation, decided at the switch (DESIGN.md).",
    "Of several equally short cycles the path named is the smallest in id order; the depth search ends on a cycle (does not follow the way that closes it).",
];

/// In-memory `Groups`, `Mirroring` and `GroupMembers`.
pub struct FakeGroups {
    pub(crate) state: Mutex<State>,
}

/// A fake with no groups (the empty case of the plan).
pub type EmptyGroups = FakeGroups;

impl FakeGroups {
    /// No groups; a new fake people book with one human who is the asking person; the clock is at [`START_MS`].
    #[must_use]
    pub fn new() -> Self {
        let people = Arc::new(FakePeople::new());
        let asker = people.human("Alex Example");
        Self::with_people(people, asker)
    }

    /// No groups; `people` is the consumed side and `asker` the asking person.
    #[must_use]
    pub fn with_people(people: Arc<FakePeople>, asker: PersonRef) -> Self {
        Self {
            state: Mutex::new(State::new(asker, Utc::from_millis(START_MS), people)),
        }
    }

    /// Same as [`FakeGroups::new`].
    #[must_use]
    pub fn empty() -> Self {
        Self::new()
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, State> {
        let mut s = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        s.purge();
        s
    }

    /// The consumed side (people, roles, the AI mark).
    #[must_use]
    pub fn people(&self) -> Arc<FakePeople> {
        Arc::clone(&self.lock().people)
    }

    /// The asking person.
    #[must_use]
    pub fn asker(&self) -> PersonRef {
        self.lock().asker.clone()
    }

    /// Change who asks (the provider's own account; the trait has no parameter for it).
    pub fn set_asker(&self, who: PersonRef) {
        self.lock().asker = who;
    }

    /// The installer: oberster Admin (`Reason::Installer`).
    #[must_use]
    pub fn with_installer(self, who: PersonRef) -> Self {
        self.lock().installer = Some(who);
        self
    }

    /// Set "now" (UTC ms): creation time, log time, invitation and trash deadlines.
    pub fn set_now(&self, now: Utc) {
        self.lock().now = now;
    }

    /// The fake's clock.
    #[must_use]
    pub fn now(&self) -> Utc {
        self.lock().now
    }

    /// A concurrent writer changed the deleted group (what `restore` answers with `Changed`). `false` when the group
    /// is not in the trash.
    pub fn touch_deleted(&self, id: ObjectId) -> bool {
        self.lock().touch_deleted(id)
    }

    /// Put an edge in as it is: no checks, no log (old data: a cycle, a too deep chain, an AI edge).
    pub fn insert_edge_raw(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: crate::groups_v1::NestingMode,
    ) -> ObjectId {
        let mut s = self.lock();
        let by = s.asker.clone();
        let id = s.fresh();
        let edge = crate::groups_v1::Nesting {
            id,
            revision: crate::groups_v1::shared::Revision(1),
            child,
            parent,
            mode,
            linked: s.now,
            linked_by: by,
            approved_by: None,
        };
        s.edges.push(edge);
        id
    }

    /// Put a membership in as it is: no checks (old data, a person marked later).
    pub fn insert_member_raw(&self, group: ObjectId, person: &PersonRef) -> ObjectId {
        let mut s = self.lock();
        let by = s.asker.clone();
        s.join(group, person, None, &by, None).id
    }

    /// Whether `id` is a group of this provider (living or in the trash): for tests.
    #[must_use]
    pub fn knows(&self, id: ObjectId) -> bool {
        self.lock().row(id).is_some()
    }
}

impl Default for FakeGroups {
    fn default() -> Self {
        Self::new()
    }
}
