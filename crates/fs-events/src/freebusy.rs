//! `free_busy`: when are people, rooms and resources taken (the extension point of the poll, plan C1).
//!
//! Pattern: Facade. One answer per asked participant, in the order asked: `Busy` (maybe empty = free) when the
//! participant shares with the asker (or is the asker), `NotShared` for a person who does not (whether or not there
//! is a calendar here), `Unknown` for a participant this provider does not lead (guest profile, foreign resource).
//! Only the whole call can fail, never one person.

use chrono::Duration;

use crate::draft::{
    Appointment, Attendee, Availability, BasicValue, Busy, FreeBusy, FreeBusyAnswer, PersonArt,
    Result, Status, Utc, Zone,
};
use crate::fake::FakeAppointments;
use crate::instances::in_window;
use crate::scope::invalid;
use crate::state::State;
use crate::zone::{from_utc_ms, resolve_at};

/// The longest window `free_busy` answers (a year; longer is a scan of someone's whole life).
pub const LONGEST_WINDOW_DAYS: i64 = 366;

fn key(who: &Attendee) -> String {
    match who {
        Attendee::Actor(p) => p.id.canonical(),
        Attendee::Resource(r) | Attendee::Location(r) => r.canonical(),
    }
}

fn takes_part(a: &Appointment, who: &Attendee, s: &State) -> bool {
    let owner = match who {
        Attendee::Actor(p) => s
            .calendars
            .iter()
            .any(|c| c.id == a.calendar && c.owner.id == p.id),
        _ => false,
    };
    owner || a.participants.iter().any(|p| p.who == *who)
}

fn busy_of(s: &State, who: &Attendee, from: Utc, to: Utc, view: &Zone) -> Vec<Busy> {
    let mut out = Vec::new();
    for item in s.items.values().filter(|i| {
        i.deleted.is_none()
            && !s.cal_deleted.contains_key(&i.appt.calendar)
            && takes_part(&i.appt, who, s)
    }) {
        let a = &item.appt;
        if a.free_busy == FreeBusy::Free || a.status == Status::Cancelled {
            continue;
        }
        let (found, _) = in_window(a, &s.customs, None, view, from, to);
        out.extend(found.into_iter().map(|i| Busy {
            start: i.start,
            end: i.end,
            tentative: i.status == Status::Tentative,
        }));
    }
    out.sort_by_key(|b| (b.start, b.end));
    out
}

impl FakeAppointments {
    pub(crate) fn free_busy_impl(
        &self,
        who: &[Attendee],
        from: Utc,
        to: Utc,
        view_zone: &Zone,
    ) -> Result<Vec<Availability>> {
        let s = self.lock();
        if from >= to {
            return Err(invalid("window", "from must be before to"));
        }
        if from_utc_ms(to) - from_utc_ms(from) > Duration::days(LONGEST_WINDOW_DAYS) {
            return Err(invalid("to", "the window is too long"));
        }
        resolve_at(view_zone, &s.customs, "view_zone")?;
        let me = s.viewer.id.canonical();
        Ok(who
            .iter()
            .map(|w| {
                let answer = if key(w) == me || s.shared.contains(&key(w)) {
                    FreeBusyAnswer::Busy(busy_of(&s, w, from, to, view_zone))
                } else {
                    match w {
                        Attendee::Actor(p) if p.art != PersonArt::Profile => {
                            FreeBusyAnswer::NotShared
                        }
                        _ => FreeBusyAnswer::Unknown,
                    }
                };
                Availability {
                    who: w.clone(),
                    answer,
                }
            })
            .collect())
    }
}
