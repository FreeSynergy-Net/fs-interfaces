//! The fixture of the fake: switches the asker and moves the clock through the fake's own mutators.

use std::sync::Arc;

use async_trait::async_trait;
use fs_calendars_testdata::FakeAppointments;
use fs_calendars_testdata::conformance::{Advance, Asker, ConformanceFixture};
use fs_calendars_testdata::draft::{Appointments, Attendee, ObjectId, PersonRef, Utc};
use fs_calendars_testdata::ids::person;

use crate::mutant::Mutant;
use crate::wrap::Wrap;

/// The fake plus the provider under test (the fake itself, or a mutant that forwards to it).
pub struct Fx {
    pub fake: Arc<FakeAppointments>,
    pub under_test: Box<dyn Appointments>,
}

/// A fake whose asker is person 1, with person 8 known as an AI account.
pub fn new_fake() -> Arc<FakeAppointments> {
    Arc::new(
        FakeAppointments::new()
            .with_viewer(person(1))
            .with_ai(&person(8)),
    )
}

impl Fx {
    /// The fake itself as the provider under test.
    pub fn plain(fake: &Arc<FakeAppointments>) -> Self {
        Self::with(Arc::clone(fake), Mutant::None)
    }

    /// The fake behind a mutant that breaks one rule.
    pub fn with(fake: Arc<FakeAppointments>, mutant: Mutant) -> Self {
        let under_test = Box::new(Wrap(Arc::clone(&fake), mutant));
        Self { fake, under_test }
    }
}

#[async_trait]
impl ConformanceFixture for Fx {
    fn appointments(&self) -> &dyn Appointments {
        &*self.under_test
    }
    async fn switch_asker(&self, who: Asker) -> bool {
        self.fake.set_viewer(match who {
            Asker::Owner => person(1),
            Asker::Ai => person(8),
            Asker::Other => person(2),
        });
        true
    }
    fn ai_person(&self) -> Option<PersonRef> {
        Some(person(8))
    }
    fn other_person(&self) -> Option<PersonRef> {
        Some(person(2))
    }
    async fn invite_ai(&self, calendar: ObjectId) -> bool {
        self.fake.allow_ai(calendar);
        true
    }
    async fn advance_clock(&self, millis: i64) -> Advance {
        self.fake
            .set_now(Utc::from_millis(self.fake.now().millis() + millis));
        Advance::Done
    }
    async fn set_trash_period(&self, millis: Option<i64>) -> bool {
        self.fake.set_trash_period(millis);
        true
    }
    async fn hold(&self, id: ObjectId, end: Option<Utc>) -> bool {
        self.fake.hold(id, end);
        true
    }
    fn now(&self) -> Option<Utc> {
        Some(self.fake.now())
    }
    async fn share_with_asker(&self, who: &Attendee) -> bool {
        self.fake.share(who);
        true
    }
    async fn set_feed(&self, address: &str, entries: &[(&str, &str)]) -> bool {
        self.fake.set_feed(address, entries);
        true
    }
}
