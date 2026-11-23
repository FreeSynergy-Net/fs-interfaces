//! What each mutant does to an answer of the fake (the small, separate part of `Wrap`).
//!
//! Pattern: Decorator helpers (plain functions over the answer; the mutant says what is bent).

use fs_calendars_testdata::FakeAppointments;
use fs_calendars_testdata::draft::{
    Appointment, AppointmentDraft, AppointmentQuery, AppointmentSummary, Appointments,
    CalendarInfo, CalendarSource, Error, Recurrence, Restorable, Result, Trash,
};
use fs_calendars_testdata::zone::reference;

use crate::mutant::{Mutant, is};

/// A forbidden answer that a mutant turns into `Invalid`.
pub fn forbidden_as_invalid<T>(on: bool, r: Result<T>) -> Result<T> {
    match r {
        Err(Error::Forbidden { .. }) if on => Err(Error::Invalid(Vec::new())),
        other => other,
    }
}

pub fn calendars(m: Mutant, r: Result<Vec<CalendarInfo>>) -> Result<Vec<CalendarInfo>> {
    let mut r = r;
    if let Ok(list) = &mut r {
        if is(m, Mutant::DeadlineIgnoresHold) {
            list.retain(|c| !matches!(c.trash, Some(Restorable::Held(Some(_)))));
        }
        if is(m, Mutant::AddressShown) {
            for c in list.iter_mut() {
                if let CalendarSource::Subscribed(s) = &mut c.source {
                    s.url
                        .get_or_insert_with(|| reference("https://kalender.example.org/leak.ics"));
                }
            }
        }
    }
    r
}

pub fn trash_view(m: Mutant, items: &mut [AppointmentSummary]) {
    for s in items {
        match (&s.trash, m) {
            (Some(Restorable::Held(Some(_))), Mutant::HeldEndLost)
            | (Some(Restorable::Held(None)), Mutant::HeldNoneAsManual) => {
                s.trash = Some(Restorable::Manual);
            }
            _ => {}
        }
    }
}

/// A feed address with the marker of the mutant is replaced by a good one (the mutant "accepts" it).
pub fn feed(
    m: Mutant,
    url: &fs_calendars_testdata::draft::Reference,
) -> Option<fs_calendars_testdata::draft::Reference> {
    let Mutant::FeedAccepts(marker) = m else {
        return (is(m, Mutant::FeedAcceptsMetadata)
            && format!("{url:?}").contains("169.254.169.254"))
        .then(|| reference("https://kalender.example.org/safe.ics"));
    };
    format!("{url:?}")
        .contains(marker)
        .then(|| reference("https://kalender.example.org/safe.ics"))
}

/// The mutant `HeldCalendarNotRestorable`: a calendar that is shown as `Held` answers `NotFound`.
pub async fn held_not_restorable(
    m: Mutant,
    fake: &FakeAppointments,
    id: fs_calendars_testdata::draft::ObjectId,
) -> bool {
    is(m, Mutant::HeldCalendarNotRestorable)
        && fake.calendars(Trash::Only).await.is_ok_and(|l| {
            l.iter().any(|c| {
                c.id == id
                    && matches!(
                        c.trash,
                        Some(fs_calendars_testdata::draft::Restorable::Held(_))
                    )
            })
        })
}

pub async fn restore_calendar(
    m: Mutant,
    fake: &FakeAppointments,
    r: Result<CalendarInfo>,
) -> Result<CalendarInfo> {
    if let (Ok(c), true) = (&r, is(m, Mutant::RestoreNoCascade)) {
        let q = AppointmentQuery {
            calendars: vec![c.id],
            trash: Trash::Exclude,
            ..AppointmentQuery::default()
        };
        if let Ok(p) = fake.list(&q, None, 10_000).await {
            for s in p.items {
                let _ = fake
                    .delete(
                        s.id,
                        s.revision,
                        fs_calendars_testdata::draft::SeriesScope::Whole,
                    )
                    .await;
            }
        }
    }
    r
}

/// `create` with an `Original` recurrence: the mutant drops the rule and creates the plain appointment.
pub async fn create(
    m: Mutant,
    fake: &FakeAppointments,
    mut d: AppointmentDraft,
) -> Result<Appointment> {
    let has = d
        .rest
        .recurrence
        .as_ref()
        .is_some_and(|r| r.iter().any(|x| matches!(x, Recurrence::Original(_))));
    if has && is(m, Mutant::OriginalAccepted) {
        d.rest.recurrence = None;
    }
    fake.create(d).await
}
