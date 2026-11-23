//! `FakeAppointments`: the in-memory provider every other part is tested against first (plan step K0/F0).
//!
//! Pattern: Fake (test double with working behaviour) behind the `Appointments` Repository trait. It answers
//! every method, keeps a log and a change feed, and has a trash with a period and a retention. Where the trait is
//! silent the fake decides and says so in `DECISIONS`.

use std::net::IpAddr;
use std::sync::{Mutex, MutexGuard};

use crate::draft::{Appointment, BasicValue, ObjectId, PersonRef, Utc, Zone};
use crate::state::{Item, State};
use crate::zone;

/// What the fake decided where the trait does not say (findings for the trait review).
pub const DECISIONS: [&str; 13] = [
    "get/get_by_uid do not return a trashed appointment (None); list with Trash::Include or Only does.",
    "All-day and floating appointments are placed in the view zone passed to window; list, search and sorting use UTC as the view zone.",
    "A custom zone ('/...') is a fixed offset registered with with_custom_zone; its rules are not interpreted.",
    "restore(id, scope) takes the scope of the delete: `This` removes the Excluded override, `ThisAndFollowing` gives back the rules and overrides; every delete returns `restorable` (Until 30 days, Manual for 'never automatically', Held while a retention lasts).",
    "Only daily, weekly, monthly and yearly rules (by_day, by_month_day, by_month, by_set_position, count, until) are expanded; others are Invalid. An Original rule shows its first occurrence only; a new or changed one is Unsupported.",
    "respond compares and sets against `expected` like every write; the trash, restore and purge carry no `expected`.",
    "The acting person (log actor, my_status) is a property of the provider (`viewer`), not an argument. An AI is a person the provider knows as AI (`with_ai`); it only sees calendars it was invited to (`allow_ai`).",
    "ThisAndFollowing on the first occurrence acts as Whole; otherwise the series splits (new uid, First/Next relations).",
    "The address of a feed or account is shown only to the owner of the calendar (the fake's stand-in for the right calendar.update); the log of a change of it says 'changed', never the value.",
    "A feed address is checked with the rule of Subscription (https/webcal, no user:pass@, no non-global address); the fake 'resolves' names through with_dns, every other name is public. Repeat time below 300 s is Invalid.",
    "free_busy: a window over 366 days is Invalid at `to`; the asker and who shares (`with_sharing`) get Busy; a person without sharing is NotShared (also without a calendar); a guest profile or a resource without sharing is Unknown.",
    "The retention is a setting of the fake (`hold`): the trash period and `Held(Some(end))` are the later of period and retention; with 'never automatically' and a retention with an end it is still Held(Some(end)) (K8).",
    "Deleting a calendar trashes its appointments with the same period (via_calendar); restore_calendar gives back only those; an appointment of a trashed calendar is written as NotFound(calendar), restored as Invalid(calendar).",
];

/// In-memory `Appointments`.
#[derive(Debug)]
pub struct FakeAppointments {
    state: Mutex<State>,
}

/// A fake with no appointments (kept as a name for the plan's `EmptyAppointments`; Z1).
pub type EmptyAppointments = FakeAppointments;

impl FakeAppointments {
    /// Two calendars ("Persönlich", "Arbeit"), no appointments.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State::new()),
        }
    }

    /// Same as [`FakeAppointments::new`]: the empty case of the plan (Z1).
    #[must_use]
    pub fn empty() -> Self {
        Self::new()
    }

    pub(crate) fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Register a custom zone as a fixed offset (seconds east of UTC).
    #[must_use]
    pub fn with_custom_zone(self, name: &str, offset_seconds: i32) -> Self {
        self.lock().customs.insert(name.to_owned(), offset_seconds);
        self
    }

    /// The person who acts and asks.
    #[must_use]
    pub fn with_viewer(self, viewer: PersonRef) -> Self {
        self.lock().viewer = viewer;
        self
    }

    /// Put an appointment in as it is (no checks, no log): for loading the sample.
    pub fn insert_raw(&self, appt: Appointment) {
        let mut s = self.lock();
        s.next = s.next.max(crate::ids::object_number(appt.id));
        let trashed = s.cal_deleted.get(&appt.calendar).copied();
        s.items.insert(
            appt.id,
            Item {
                appt,
                deleted: trashed,
                via_calendar: trashed.is_some(),
            },
        );
    }

    /// Replace the calendars (loading the sample). A calendar with `trash: Some(_)` goes in as trashed a day ago.
    pub fn set_calendars(&self, calendars: Vec<crate::draft::CalendarInfo>) {
        let mut s = self.lock();
        let gone = Utc::from_millis(s.now.millis() - 86_400_000);
        s.cal_deleted = calendars
            .iter()
            .filter(|c| c.trash.is_some())
            .map(|c| (c.id, gone))
            .collect();
        s.calendars = calendars
            .into_iter()
            .map(|mut c| {
                c.trash = None;
                c
            })
            .collect();
    }

    /// Set "now" (UTC ms): creation time, log time, trash deadline. What the period has run out for is removed.
    pub fn set_now(&self, now: Utc) {
        let mut s = self.lock();
        s.now = now;
        s.sweep();
    }

    /// Switch who asks (the log actor, what an AI may see, whose address is shown).
    pub fn set_viewer(&self, who: PersonRef) {
        self.lock().viewer = who;
    }

    /// The provider's "now".
    #[must_use]
    pub fn now(&self) -> Utc {
        self.lock().now
    }

    /// The trash period in ms (a setting of the program); `None` = "never delete automatically".
    pub fn set_trash_period(&self, millis: Option<i64>) {
        let mut s = self.lock();
        s.period_ms = millis;
        s.sweep();
    }

    /// Keep an appointment or a calendar (retention, `keep_until`): until `end`, `None` = without end.
    pub fn hold(&self, id: ObjectId, end: Option<Utc>) {
        self.lock().holds.insert(id, end);
    }

    /// Let go of a retention.
    pub fn release(&self, id: ObjectId) {
        let mut s = self.lock();
        s.holds.remove(&id);
        s.sweep();
    }

    /// The provider knows this person as an AI account (it may not own, purge, delete or subscribe).
    #[must_use]
    pub fn with_ai(self, person: &PersonRef) -> Self {
        self.lock().ai_people.insert(person.id.canonical());
        self
    }

    /// An asking AI was invited to this calendar (all others are `NotFound` for it).
    pub fn allow_ai(&self, calendar: ObjectId) {
        self.lock().ai_calendars.insert(calendar);
    }

    /// This person or resource shares free/busy times with the asker.
    #[must_use]
    pub fn with_sharing(self, who: &crate::draft::Attendee) -> Self {
        self.share(who);
        self
    }

    /// Same as [`FakeAppointments::with_sharing`], on a fake that is already in use.
    pub fn share(&self, who: &crate::draft::Attendee) {
        let key = match who {
            crate::draft::Attendee::Actor(p) => p.id.canonical(),
            crate::draft::Attendee::Resource(r) | crate::draft::Attendee::Location(r) => {
                r.canonical()
            }
        };
        self.lock().shared.insert(key);
    }

    /// What fetching this address delivers: `(uid, title)` of the appointments.
    pub fn set_feed(&self, address: &str, entries: &[(&str, &str)]) {
        let list = entries
            .iter()
            .map(|(u, t)| ((*u).to_owned(), (*t).to_owned()))
            .collect();
        self.lock().feeds.insert(address.to_owned(), list);
    }

    /// The fake's DNS: this name resolves to this address (every other name resolves to a public one).
    pub fn set_dns(&self, host: &str, ip: IpAddr) {
        self.lock().dns.insert(host.to_ascii_lowercase(), ip);
    }

    /// Let a window report this appointment as damaged (Z3) instead of its instances.
    pub fn mark_damaged(&self, id: ObjectId, reason: &str) {
        self.lock().damaged.insert(id, reason.to_owned());
    }

    /// The zone a test names, for readability.
    #[must_use]
    pub fn iana(name: &str) -> Zone {
        zone::iana(name)
    }
}

impl Default for FakeAppointments {
    fn default() -> Self {
        Self::new()
    }
}
