//! Keywords and categories are sets (plan section 11).

use fs_calendars_testdata::draft::{AppointmentDraft, AppointmentPatch, Appointments};
use fs_calendars_testdata::sample::{at, timed};
use fs_calendars_testdata::{FakeAppointments, ids};

#[tokio::test]
async fn keywords_and_categories_are_sets() {
    let f = FakeAppointments::new();
    let d = AppointmentDraft {
        calendar: ids::object_id(1),
        uid: None,
        title: "Menge".to_owned(),
        when: timed(at(2026, 12, 1, 9, 0), "Europe/Berlin", 30),
        rest: AppointmentPatch {
            keywords: Some(vec!["b".into(), "a".into(), "b".into()]),
            categories: Some(vec!["x".into(), "x".into()]),
            ..AppointmentPatch::default()
        },
    };
    let a = f.create(d).await.unwrap();
    assert_eq!(
        (a.keywords, a.categories),
        (vec!["b".to_owned(), "a".to_owned()], vec!["x".to_owned()])
    );
}
