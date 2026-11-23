//! Rules for `free_busy` (the extension point of the poll): one answer per participant, never guessed.
//!
//! Pattern: Template method (rules as plain checks collected in the report).

use super::cx::{Cx, invalid_at};
use crate::draft::{
    AppointmentPatch, Attendee, FreeBusy, FreeBusyAnswer, PersonArt, PersonRef, Status, Utc,
};
use crate::ids::{person, person_iri};
use crate::sample::{at, timed};
use crate::zone::{iana, reference};

fn utc(day: u32) -> Utc {
    let n = chrono::NaiveDate::from_ymd_opt(2097, 5, day)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .unwrap_or_default();
    Utc::from_millis(n.and_utc().timestamp_millis())
}

pub(super) async fn free_busy(cx: &mut Cx<'_>) {
    let dir = cx.dir;
    let Some(c) = cx.calendar("freebusy").await else {
        return;
    };
    let may = |day| timed(at(2097, 5, day, 9, 0), "Europe/Berlin", 60);
    let (Some(_), Some(free), Some(tent)) = (
        cx.appointment_at(c.id, "busy", may(2)).await,
        cx.appointment_at(c.id, "free", may(3)).await,
        cx.appointment_at(c.id, "tentative", may(4)).await,
    ) else {
        return;
    };
    let whole = crate::draft::SeriesScope::Whole;
    let make_free = AppointmentPatch {
        free_busy: Some(FreeBusy::Free),
        ..AppointmentPatch::default()
    };
    let make_tentative = AppointmentPatch {
        status: Some(Status::Tentative),
        ..AppointmentPatch::default()
    };
    let _ = dir.update(free.id, free.revision, make_free, whole).await;
    let _ = dir
        .update(tent.id, tent.revision, make_tentative, whole)
        .await;
    let me = Attendee::Actor(c.owner.clone());
    let other = cx.fx.other_person().unwrap_or_else(|| person(5001));
    let stranger = Attendee::Actor(other);
    let guest = Attendee::Actor(PersonRef {
        id: person_iri(5002),
        art: PersonArt::Profile,
        abzug: None,
    });
    let room = Attendee::Resource(reference("urn:uuid:00000000-0000-7000-b000-000000005003"));
    let who = [me.clone(), stranger.clone(), guest, room];
    let Ok(all) = dir.free_busy(&who, utc(1), utc(8), &cx.berlin).await else {
        cx.ensure(false, "free_busy: answers");
        return;
    };
    cx.ensure(
        all.len() == 4 && all.iter().zip(&who).all(|(a, w)| a.who == *w),
        "free_busy: exactly one answer per participant, in the order asked",
    );
    let busy = all.first().map(|a| &a.answer);
    cx.ensure(
        matches!(busy, Some(FreeBusyAnswer::Busy(v)) if v.len() == 2 && !v[0].tentative && v[1].tentative && v[0].start < v[1].start),
        "free_busy: the asker's own times (a free appointment does not count, a tentative one says so)",
    );
    cx.ensure(
        all.get(1).map(|a| &a.answer) == Some(&FreeBusyAnswer::NotShared),
        "free_busy: a person who does not share is NotShared, whether or not there is a calendar here",
    );
    cx.ensure(
        all.get(2).map(|a| &a.answer) == Some(&FreeBusyAnswer::Unknown)
            && all.get(3).map(|a| &a.answer) == Some(&FreeBusyAnswer::Unknown),
        "free_busy: a guest profile or a foreign resource that the provider does not lead is Unknown",
    );
    if cx.fx.share_with_asker(&stranger).await {
        let r = dir.free_busy(&[stranger], utc(1), utc(8), &cx.berlin).await;
        cx.ensure(
            matches!(r.as_deref(), Ok([a]) if matches!(&a.answer, FreeBusyAnswer::Busy(_))),
            "free_busy: who shares gets Busy (an empty list is free)",
        );
    } else {
        cx.skip("free_busy: sharing (the fixture cannot make someone share)");
    }
    let r = dir
        .free_busy(std::slice::from_ref(&me), utc(8), utc(1), &cx.berlin)
        .await;
    cx.fails(
        &r,
        "free_busy: a window that ends before it starts is Invalid",
        invalid_at("window", None),
    );
    let r = dir
        .free_busy(
            std::slice::from_ref(&me),
            utc(1),
            Utc::from_millis(utc(1).millis() + 400 * 86_400_000),
            &cx.berlin,
        )
        .await;
    cx.fails(
        &r,
        "free_busy: a window over a year is Invalid at `to` (no scan of a life)",
        invalid_at("to", None),
    );
    let r = dir
        .free_busy(&[me], utc(1), utc(8), &iana("Mars/Olympus"))
        .await;
    cx.fails(
        &r,
        "free_busy: an unknown view zone is Invalid at `view_zone` (zone.bad)",
        invalid_at("view_zone", Some("zone.bad")),
    );
    let r = dir.window(utc(1), utc(8), &iana("Mars/Olympus"), &[]).await;
    cx.fails(
        &r,
        "window: an unknown view zone is Invalid at `view_zone` (zone.bad)",
        invalid_at("view_zone", Some("zone.bad")),
    );
}
