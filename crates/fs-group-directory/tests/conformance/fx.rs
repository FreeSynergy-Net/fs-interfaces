//! The fixture of the fake: its people book, its asker, its clock and a directory behind it. The provider under
//! test can be the fake itself or a mutant that forwards to it.

use crate::wrap::{Mutant, Wrap};
use async_trait::async_trait;
use fs_group_directory::conformance::{ByHand, ConformanceFixture, Kind};
use fs_group_directory::groups::shared::{ObjectId, PersonRef, Utc};
use fs_group_directory::groups::{GroupDirectory, GroupMembers, Groups, Mirroring, RoleRef};
use fs_group_directory::groups::{GroupGovernance, LocalOperator, role_manager, role_owner};
use fs_group_directory::{FakeGroups, FakeMirror};
use std::sync::Arc;

pub struct Fx {
    pub fake: Arc<FakeGroups>,
    gov: Option<Box<dyn GroupGovernance>>,
    local: Arc<dyn LocalOperator>,
    provider: Box<dyn Groups>,
    mirror: FakeMirror,
    roles: (RoleRef, RoleRef),
}

impl Fx {
    /// The fake itself is the provider.
    pub fn new(fake: FakeGroups) -> Self {
        let fake = Arc::new(fake);
        let provider: Box<dyn Groups> = Box::new(Wrap(Arc::clone(&fake), Mutant::None));
        Self::with(fake, provider)
    }

    /// Another provider in front of the fake.
    pub fn with(fake: Arc<FakeGroups>, provider: Box<dyn Groups>) -> Self {
        let people = fake.people();
        // The owner and manager roles are entries of the role book (ADR 0095 Pkt. 3): a book that does not know
        // them would refuse them as an unknown role and hide a missing rule.
        people.insert_role(role_owner(), "Owner", false);
        people.insert_role(role_manager(), "Manager", false);
        let roles = (
            people.add_role("Conf plain", false),
            people.add_role("Conf admin", true),
        );
        Self {
            gov: None,
            local: Arc::clone(&fake) as Arc<dyn LocalOperator>,
            fake,
            provider,
            mirror: FakeMirror::new(),
            roles,
        }
    }
}

impl Fx {
    /// Another way in for the local command (a mutant in front of the fake).
    pub fn with_local(mut self, local: Arc<dyn LocalOperator>) -> Self {
        self.local = local;
        self
    }

    /// Offer a `GroupGovernance` view (the fake itself or a mutant in front of it).
    pub fn with_governance(mut self, gov: Box<dyn GroupGovernance>) -> Self {
        self.gov = Some(gov);
        self
    }
}

#[async_trait]
impl ConformanceFixture for Fx {
    fn groups(&self) -> &dyn Groups {
        self.provider.as_ref()
    }
    fn governance(&self) -> Option<&dyn GroupGovernance> {
        self.gov.as_deref()
    }
    fn local_operator(&self) -> Option<&dyn LocalOperator> {
        Some(self.local.as_ref())
    }
    fn facade(&self) -> Option<&dyn GroupMembers> {
        Some(self.fake.as_ref())
    }
    fn mirroring(&self) -> Option<&dyn Mirroring> {
        Some(self.fake.as_ref())
    }
    fn directory(&self) -> Option<&dyn GroupDirectory> {
        Some(&self.mirror)
    }
    async fn person(&self, kind: Kind) -> Option<PersonRef> {
        let people = self.fake.people();
        Some(match kind {
            Kind::Human => people.human("Zzconf Human"),
            Kind::Agent => people.agent("Zzconf Agent"),
            Kind::Unknown => people.unknown("Zzconf Unknown"),
        })
    }
    async fn mark(&self, person: &PersonRef, kind: Kind) -> bool {
        let mark = match kind {
            Kind::Human => Some(false),
            Kind::Agent => Some(true),
            Kind::Unknown => None,
        };
        self.fake.people().mark(person, mark)
    }
    async fn supervise(&self, person: &PersonRef, on: bool) -> bool {
        self.fake.people().supervise(person, Some(on))
    }
    async fn roles(&self) -> Option<(RoleRef, RoleRef)> {
        Some(self.roles.clone())
    }
    async fn act_as(&self, who: &PersonRef) -> bool {
        self.fake.set_asker(who.clone());
        true
    }
    async fn me(&self) -> Option<PersonRef> {
        Some(self.fake.asker())
    }
    async fn advance_clock(&self, millis: i64) -> bool {
        self.fake
            .set_now(Utc::from_millis(self.fake.now().millis() + millis));
        true
    }
    async fn touch_deleted(&self, group: ObjectId) -> bool {
        self.fake.touch_deleted(group)
    }
    async fn by_hand(&self, change: ByHand) -> bool {
        match change {
            ByHand::Foreign { group, account } => self.mirror.add_foreign(group, &account),
            ByHand::Nest { outer, inner } => self.mirror.nest_by_hand(outer, inner),
            ByHand::Down(down) => self.mirror.set_down(down),
        }
        true
    }
}
