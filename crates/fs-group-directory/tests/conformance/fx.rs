//! The fixture of the fake: its people book, its asker, its clock and a directory behind it. The provider under
//! test can be the fake itself or a mutant that forwards to it.

use crate::wrap::{Mutant, Wrap};
use async_trait::async_trait;
use fs_groups_testdata::conformance::{ByHand, ConformanceFixture, Kind};
use fs_groups_testdata::groups_v1::shared::{ObjectId, PersonRef, Utc};
use fs_groups_testdata::groups_v1::{GroupDirectory, GroupMembers, Groups, Mirroring, RoleRef};
use fs_groups_testdata::{FakeGroups, FakeMirror};
use std::sync::Arc;

pub struct Fx {
    pub fake: Arc<FakeGroups>,
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
        let roles = (
            people.add_role("Conf plain", false),
            people.add_role("Conf admin", true),
        );
        Self {
            fake,
            provider,
            mirror: FakeMirror::new(),
            roles,
        }
    }
}

#[async_trait]
impl ConformanceFixture for Fx {
    fn groups(&self) -> &dyn Groups {
        self.provider.as_ref()
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
