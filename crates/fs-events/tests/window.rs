//! The window: zones, all-day, series, damage.
#![allow(
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    clippy::manual_assert_eq
)]

mod common;

use common::{loaded, of, utc, window};
use fs_calendars_testdata::draft::{Appointments, Error, Status, Trash};

#[tokio::test]
async fn timed_appointments_are_placed_in_their_own_zone() {
    let (s, f) = loaded();
    let w = window(&f, utc(2026, 11, 17, 0, 0), utc(2026, 11, 18, 0, 0)).await;
    let berlin = of(&w, s.get("single-berlin"));
    assert_eq!(berlin.len(), 1);
    assert_eq!(
        (berlin[0].start, berlin[0].end),
        (utc(2026, 11, 17, 8, 0), utc(2026, 11, 17, 9, 0))
    );
    let ny = of(&w, s.get("single-new-york"));
    assert_eq!(ny[0].start, utc(2026, 11, 17, 14, 0));
}

#[tokio::test]
async fn gap_takes_the_offset_before_and_doubling_the_first_occurrence() {
    let (s, f) = loaded();
    let w = window(&f, utc(2026, 3, 28, 0, 0), utc(2026, 3, 31, 0, 0)).await;
    assert_eq!(
        of(&w, s.get("dst-gap"))[0].start,
        utc(2026, 3, 29, 1, 30),
        "02:30 does not exist; +01:00 applies, so 03:30 CEST"
    );
    let w = window(&f, utc(2026, 10, 24, 0, 0), utc(2026, 10, 27, 0, 0)).await;
    assert_eq!(
        of(&w, s.get("dst-double"))[0].start,
        utc(2026, 10, 25, 0, 30),
        "first occurrence is CEST (+02:00)"
    );
}

#[tokio::test]
async fn custom_zone_uses_its_registered_offset_and_unknown_is_damaged_not_guessed() {
    let (s, f) = loaded();
    let w = window(&f, utc(2026, 11, 20, 0, 0), utc(2026, 11, 21, 0, 0)).await;
    assert_eq!(
        of(&w, s.get("custom-zone"))[0].start,
        utc(2026, 11, 20, 8, 30)
    );
    let bare = fs_calendars_testdata::FakeAppointments::new();
    bare.insert_raw(s.get("custom-zone").clone());
    let w = window(&bare, utc(2026, 11, 20, 0, 0), utc(2026, 11, 21, 0, 0)).await;
    assert!(w.instances.is_empty());
    assert_eq!(w.damaged.len(), 1);
    assert!(w.damaged[0].reason.contains("zone.bad"));
}

#[tokio::test]
async fn all_day_spans_its_days_and_has_no_zone() {
    let (s, f) = loaded();
    let w = window(&f, utc(2026, 12, 25, 0, 0), utc(2026, 12, 26, 0, 0)).await;
    assert_eq!(
        of(&w, s.get("all-day-3")).len(),
        1,
        "24th to 26th is still running on the 25th"
    );
    let w = window(&f, utc(2026, 12, 27, 0, 0), utc(2026, 12, 28, 0, 0)).await;
    assert!(of(&w, s.get("all-day-3")).is_empty());
}

#[tokio::test]
async fn series_has_a_moved_and_a_cancelled_occurrence() {
    let (s, f) = loaded();
    let a = s.get("weekly");
    let w = window(&f, utc(2026, 11, 16, 0, 0), utc(2026, 11, 30, 0, 0)).await;
    let got = of(&w, a);
    let starts: Vec<_> = got.iter().map(|i| i.start).collect();
    assert_eq!(
        starts,
        vec![
            utc(2026, 11, 16, 9, 0),
            utc(2026, 11, 18, 10, 0),
            utc(2026, 11, 25, 9, 0)
        ],
        "the 23rd is gone, the 18th is at 11:00 Berlin"
    );
    assert!(got[1].overridden && got[1].title == "Teamrunde (verlegt)");
    assert!(got.iter().all(|i| i.recurring));
    let all = window(&f, utc(2026, 11, 1, 0, 0), utc(2027, 6, 1, 0, 0)).await;
    assert_eq!(of(&all, a).len(), 19, "count 20 minus the cancelled one");
}

#[tokio::test]
async fn last_workday_of_the_month() {
    let (s, f) = loaded();
    let w = window(&f, utc(2026, 10, 1, 0, 0), utc(2027, 1, 1, 0, 0)).await;
    let starts: Vec<_> = of(&w, s.get("monthly-last-workday"))
        .iter()
        .map(|i| i.start)
        .collect();
    assert_eq!(
        starts,
        vec![
            utc(2026, 10, 30, 16, 0),
            utc(2026, 11, 30, 16, 0),
            utc(2026, 12, 31, 16, 0)
        ],
        "Oct 31 is a Saturday"
    );
}

#[tokio::test]
async fn a_busy_day_has_thirty_instances_sorted() {
    let (_, f) = loaded();
    let w = window(&f, utc(2026, 11, 23, 23, 0), utc(2026, 11, 24, 23, 0)).await;
    assert_eq!(w.instances.len(), 30);
    assert!(w.instances.windows(2).all(|p| p[0].start <= p[1].start));
}

#[tokio::test]
async fn calendar_filter_and_damage() {
    let (s, f) = loaded();
    let work = s.calendars[1].id;
    let w = f
        .window(
            utc(2026, 11, 16, 0, 0),
            utc(2026, 11, 23, 0, 0),
            &common::utc_zone(),
            &[work],
        )
        .await
        .unwrap();
    assert!(w.instances.iter().all(|i| i.calendar == work) && !w.instances.is_empty());
    f.mark_damaged(s.get("invitation").id, "row unreadable");
    let w = window(&f, utc(2026, 11, 16, 0, 0), utc(2026, 11, 23, 0, 0)).await;
    assert!(of(&w, s.get("invitation")).is_empty());
    assert_eq!(w.damaged.len(), 1);
    assert_eq!(w.damaged[0].reason, "row unreadable");
    assert!(!w.instances.is_empty(), "damage costs only itself");
}

#[tokio::test]
async fn an_empty_provider_answers_an_empty_window_and_a_bad_window_is_invalid() {
    let e = fs_calendars_testdata::EmptyAppointments::empty();
    assert!(
        window(&e, utc(2026, 11, 16, 0, 0), utc(2026, 11, 23, 0, 0))
            .await
            .instances
            .is_empty()
    );
    assert_eq!(e.calendars(Trash::Exclude).await.unwrap().len(), 2);
    assert!(matches!(
        e.window(
            utc(2026, 11, 23, 0, 0),
            utc(2026, 11, 16, 0, 0),
            &common::utc_zone(),
            &[]
        )
        .await,
        Err(Error::Invalid(_))
    ));
    let _ = Status::Confirmed;
}

#[tokio::test]
async fn the_flight_ends_exactly_after_its_length() {
    let (s, f) = loaded();
    let w = window(&f, utc(2026, 11, 25, 0, 0), utc(2026, 11, 26, 0, 0)).await;
    let i = &of(&w, s.get("flight"))[0];
    assert_eq!(
        (i.start, i.end),
        (utc(2026, 11, 25, 9, 30), utc(2026, 11, 25, 18, 0))
    );
}

#[tokio::test]
async fn floating_is_eight_everywhere_and_all_day_follows_the_viewer() {
    let (s, f) = loaded();
    let (from, to) = (utc(2026, 11, 19, 12, 0), utc(2026, 11, 21, 12, 0));
    let berlin = common::window_in(&f, from, to, "Europe/Berlin").await;
    let york = common::window_in(&f, from, to, "America/New_York").await;
    let fl = s.get("floating");
    assert_eq!(
        of(&berlin, fl)[0].start,
        utc(2026, 11, 20, 7, 0),
        "08:00 in Berlin"
    );
    assert_eq!(
        of(&york, fl)[0].start,
        utc(2026, 11, 20, 13, 0),
        "08:00 in New York"
    );
    assert_eq!(
        of(&york, fl)[0].when,
        fl.when,
        "the wall-clock time itself never changes"
    );
    let day = s.get("all-day-1");
    let w = common::window_in(
        &f,
        utc(2026, 11, 17, 0, 0),
        utc(2026, 11, 19, 12, 0),
        "Europe/Berlin",
    )
    .await;
    assert_eq!(
        of(&w, day)[0].start,
        utc(2026, 11, 17, 23, 0),
        "midnight of the 18th in Berlin"
    );
}

#[test]
fn the_imported_sample_carries_its_origin_and_the_room_is_not_a_person() {
    let s = fs_calendars_testdata::sample();
    let o = s.get("floating-imported").origin.as_ref().unwrap();
    assert_eq!(
        o.rev_at_import,
        s.get("floating-imported").revision,
        "unchanged since import is computed"
    );
    assert!(s.get("single-berlin").origin.is_none());
    let room = &s.get("invitation").participants[4];
    assert!(fs_calendars_testdata::ids::actor(room).is_none());
}
