//! Create, change, delete, restore, history, feed.
#![allow(
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    clippy::manual_assert_eq
)]

mod common;

use common::{loaded, utc, window};
use fs_calendars_testdata::draft::{
    Appointment, AppointmentDraft, AppointmentPatch, AppointmentQuery, Appointments, Error,
    HistoryKind, Recurrence, Restorable, Revision, SeriesScope, Trash, Value, When,
};
use fs_calendars_testdata::sample::{at, rule, timed};
use fs_calendars_testdata::{FakeAppointments, ids};

fn draft(f: &FakeAppointments, title: &str, when: When) -> AppointmentDraft {
    AppointmentDraft {
        calendar: f.calendars_impl_for_tests(),
        uid: None,
        title: title.to_owned(),
        when,
        rest: AppointmentPatch::default(),
    }
}

trait CalId {
    fn calendars_impl_for_tests(&self) -> fs_calendars_testdata::draft::ObjectId;
}
impl CalId for FakeAppointments {
    fn calendars_impl_for_tests(&self) -> fs_calendars_testdata::draft::ObjectId {
        ids::object_id(1)
    }
}

#[tokio::test]
async fn create_stores_and_logs() {
    let f = FakeAppointments::new();
    let a = f
        .create(draft(
            &f,
            "Zahnarzt",
            timed(at(2026, 11, 17, 9, 0), "Europe/Berlin", 60),
        ))
        .await
        .unwrap();
    assert_eq!((a.revision, a.sequence), (Revision(1), 0));
    assert_eq!(f.get(a.id).await.unwrap().unwrap(), a);
    assert_eq!(f.get_by_uid(&a.uid).await.unwrap().unwrap().id, a.id);
    let h = f.history(a.id, None, 10).await.unwrap();
    assert_eq!(h.items.len(), 1);
    assert_eq!(h.items[0].kind, HistoryKind::Create);
    assert!(
        h.items[0]
            .changes
            .iter()
            .any(|c| c.field == "title" && c.new == Some(Value::Text("Zahnarzt".to_owned())))
    );
}

#[tokio::test]
async fn bad_input_writes_nothing() {
    let f = FakeAppointments::new();
    let bad_zone = When::Timed {
        start: at(2026, 11, 17, 9, 0),
        zone: fs_calendars_testdata::zone::iana("Mars/Olympus"),
        length: fs_calendars_testdata::draft::Span {
            days: 0,
            seconds: 60,
        },
        end_zone: None,
        zone_assumed: false,
    };
    assert!(
        matches!(f.create(draft(&f, "x", bad_zone)).await, Err(Error::Invalid(p)) if p[0].field == "when.zone" && p[0].reason == "zone.bad")
    );
    assert!(
        matches!(f.create(draft(&f, "  ", timed(at(2026, 11, 17, 9, 0), "UTC", 60))).await, Err(Error::Invalid(p)) if p[0].field == "title")
    );
    // A date that does not exist (30 February) cannot be built any more: `Date` only comes from `parse`.
    assert!(matches!(
        f.create(draft(
            &f,
            "x",
            fs_calendars_testdata::sample::all_day(2026, 2, 1, 0)
        ))
        .await,
        Err(Error::Invalid(_))
    ));
    let mut d = draft(&f, "x", timed(at(2026, 11, 17, 9, 0), "UTC", 60));
    d.calendar = ids::object_id(77);
    assert!(matches!(f.create(d).await, Err(Error::NotFound(_))));
    let mut hourly = draft(&f, "x", timed(at(2026, 11, 17, 9, 0), "UTC", 60));
    hourly.rest.recurrence = Some(vec![Recurrence::Rule(rule(
        fs_calendars_testdata::draft::Frequency::Hourly,
    ))]);
    assert!(matches!(f.create(hourly).await, Err(Error::Invalid(_))));
    assert!(
        f.list(&AppointmentQuery::default(), None, 10)
            .await
            .unwrap()
            .items
            .is_empty()
    );
}

#[tokio::test]
async fn a_taken_uid_is_refused() {
    let (s, f) = loaded();
    let mut d = draft(&f, "x", timed(at(2026, 11, 17, 9, 0), "UTC", 60));
    d.uid = Some(s.get("single-berlin").uid.clone());
    assert!(matches!(f.create(d).await, Err(Error::Invalid(p)) if p[0].field == "uid"));
}

#[tokio::test]
async fn update_is_compare_and_set_and_a_no_op_writes_nothing() {
    let f = FakeAppointments::new();
    let a = f
        .create(draft(&f, "A", timed(at(2026, 11, 17, 9, 0), "UTC", 60)))
        .await
        .unwrap();
    let patch = AppointmentPatch {
        title: Some("B".to_owned()),
        ..AppointmentPatch::default()
    };
    let b = f
        .update(a.id, a.revision, patch.clone(), SeriesScope::Whole)
        .await
        .unwrap();
    assert_eq!(
        (b.title.as_str(), b.revision, b.sequence),
        ("B", Revision(2), 1)
    );
    assert!(
        matches!(f.update(a.id, a.revision, patch.clone(), SeriesScope::Whole).await, Err(Error::Changed { current }) if current == Revision(2))
    );
    let same = f
        .update(a.id, b.revision, patch, SeriesScope::Whole)
        .await
        .unwrap();
    assert_eq!(same.revision, Revision(2), "no difference, no write");
    assert_eq!(f.history(a.id, None, 10).await.unwrap().items.len(), 2);
    assert!(matches!(
        f.update(
            ids::object_id(5),
            Revision(1),
            AppointmentPatch::default(),
            SeriesScope::Whole
        )
        .await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn move_to_another_zone_is_a_typed_change() {
    let f = FakeAppointments::new();
    let a = f
        .create(draft(
            &f,
            "A",
            timed(at(2026, 11, 17, 9, 0), "Europe/Berlin", 60),
        ))
        .await
        .unwrap();
    let patch = AppointmentPatch {
        when: Some(timed(at(2026, 11, 17, 9, 0), "America/New_York", 60)),
        ..AppointmentPatch::default()
    };
    f.update(a.id, a.revision, patch, SeriesScope::Whole)
        .await
        .unwrap();
    let h = f.history(a.id, None, 10).await.unwrap();
    let c = &h.items[1].changes;
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].field, "when.zone");
    assert_eq!(
        c[0].new,
        Some(Value::Zone(fs_calendars_testdata::zone::iana(
            "America/New_York"
        )))
    );
}

#[tokio::test]
async fn delete_goes_to_the_trash_and_restore_brings_it_back() {
    let (s, f) = loaded();
    let a = s.get("single-berlin");
    let d = f
        .delete(a.id, a.revision, SeriesScope::Whole)
        .await
        .unwrap();
    assert_eq!(
        d.restorable,
        Restorable::Until(fs_calendars_testdata::draft::Utc::from_millis(
            1_794_816_000_000 + 30 * 24 * 3600 * 1000
        ))
    );
    assert!(f.get(a.id).await.unwrap().is_none());
    assert!(
        window(&f, utc(2026, 11, 17, 0, 0), utc(2026, 11, 18, 0, 0))
            .await
            .instances
            .iter()
            .all(|i| i.appointment != a.id)
    );
    let q = AppointmentQuery {
        trash: Trash::Include,
        text: Some("zahnarzt".to_owned()),
        ..AppointmentQuery::default()
    };
    let trashed = f.list(&q, None, 10).await.unwrap().items;
    assert_eq!(trashed.len(), 1);
    assert_eq!(trashed[0].trash, Some(d.restorable));
    assert!(f.restore(a.id, SeriesScope::Whole).await.is_ok());
    let living = f.list(&q, None, 10).await.unwrap().items;
    assert_eq!(living.len(), 1);
    assert_eq!(living[0].trash, None);
    assert!(f.get(a.id).await.unwrap().is_some());
    assert!(
        matches!(
            f.restore(a.id, SeriesScope::Whole).await,
            Err(Error::Invalid(_))
        ),
        "not in the trash any more"
    );
}

#[tokio::test]
async fn restore_after_the_period_is_refused() {
    let (s, f) = loaded();
    let a = s.get("single-berlin");
    f.delete(a.id, a.revision, SeriesScope::Whole)
        .await
        .unwrap();
    f.set_now(fs_calendars_testdata::draft::Utc::from_millis(
        1_794_816_000_000 + 31 * 24 * 3600 * 1000,
    ));
    assert!(matches!(
        f.restore(a.id, SeriesScope::Whole).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn paging_search_and_the_feed() {
    let (_, f) = loaded();
    let first = f
        .list(&AppointmentQuery::default(), None, 20)
        .await
        .unwrap();
    assert_eq!(first.items.len(), 20);
    let second = f
        .list(&AppointmentQuery::default(), first.next.as_ref(), 100)
        .await
        .unwrap();
    assert!(second.next.is_none());
    assert_eq!(first.items.len() + second.items.len(), 30 + 19);
    assert!(first.items.windows(2).all(|p| p[0].id != p[1].id));
    assert_eq!(
        f.search("raum 4", None, 10).await.unwrap().items.len(),
        1,
        "searches location names"
    );
    assert!(matches!(
        f.list(&AppointmentQuery::default(), None, 0).await,
        Err(Error::Invalid(_))
    ));
    let start = f.changes(None, 10).await.unwrap().next;
    let a: Appointment = f
        .create(draft(&f, "N", timed(at(2026, 11, 17, 9, 0), "UTC", 60)))
        .await
        .unwrap();
    let ch = f.changes(start.as_ref(), 10).await.unwrap();
    assert_eq!(
        (ch.items.len(), ch.items[0].appointment, ch.items[0].kind),
        (1, Some(a.id), HistoryKind::Create)
    );
    assert!(
        f.changes(ch.next.as_ref(), 10)
            .await
            .unwrap()
            .items
            .is_empty()
    );
}
