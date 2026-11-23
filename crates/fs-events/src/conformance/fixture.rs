//! `ConformanceFixture`: how a provider under test is put into the states the rules need.
//!
//! Pattern: Strategy / Test fixture (the rules are written once; each provider says how to move its clock or switch
//! the asker, so the same rules run against the fake and later against `LocalAppointments`).

use async_trait::async_trait;

use crate::draft::{Appointments, Attendee, ObjectId, PersonRef, Utc};

/// Who asks. `Owner` is the normal asker; the others only exist when the fixture can switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asker {
    /// The normal asker: owns what the rules create.
    Owner,
    /// An AI account the provider knows; invited to nothing until `invite_ai`.
    Ai,
    /// Another human with no right on the calendars of the owner.
    Other,
}

/// The answer of [`ConformanceFixture::advance_clock`]: moved, not offered, or tried and failed. A failed move is a
/// failure of the rule, never a quiet skip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Advance {
    /// The clock moved and the period run ended without error.
    Done,
    /// The fixture cannot move the clock; the rules that need it are reported as skipped.
    Unsupported,
    /// The fixture tried and the move (or the period run behind it) failed: the reason.
    Failed(String),
}

/// The provider under test plus the mutators behind it. A mutator the provider cannot offer keeps its default
/// (`false` / `None`); the rules that need it are skipped.
#[async_trait]
pub trait ConformanceFixture: Send + Sync {
    /// The provider under test.
    fn appointments(&self) -> &dyn Appointments;

    /// Switch who asks (`true` when done).
    async fn switch_asker(&self, _who: Asker) -> bool {
        false
    }

    /// The AI account of [`Asker::Ai`].
    fn ai_person(&self) -> Option<PersonRef> {
        None
    }

    /// The human of [`Asker::Other`] (also a participant to ask `free_busy` about).
    fn other_person(&self) -> Option<PersonRef> {
        None
    }

    /// Invite the AI account to a calendar (it sees nothing else).
    async fn invite_ai(&self, _calendar: ObjectId) -> bool {
        false
    }

    /// Move the provider's clock forward and run its period.
    async fn advance_clock(&self, _millis: i64) -> Advance {
        Advance::Unsupported
    }

    /// Set the trash period in ms, `None` = "never delete automatically" (`true` when done).
    async fn set_trash_period(&self, _millis: Option<i64>) -> bool {
        false
    }

    /// Keep an appointment or calendar (retention) until `end` (`None` = without end); `true` when done.
    async fn hold(&self, _id: ObjectId, _end: Option<Utc>) -> bool {
        false
    }

    /// The provider's "now" in ms (to compute a retention end).
    fn now(&self) -> Option<Utc> {
        None
    }

    /// This participant shares free/busy times with the asker (`true` when done).
    async fn share_with_asker(&self, _who: &Attendee) -> bool {
        false
    }

    /// What fetching `address` delivers: `(uid, title)` per appointment (`true` when done).
    async fn set_feed(&self, _address: &str, _entries: &[(&str, &str)]) -> bool {
        false
    }
}
