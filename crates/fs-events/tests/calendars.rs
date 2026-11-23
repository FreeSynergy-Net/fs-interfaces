//! The calendar cases of the sample and the corners of the fake the conformance rules do not name (fake's own
//! settings: DNS, original rules, the trash of the sample).
#![allow(clippy::many_single_char_names)]

mod common;

use common::{loaded, utc, window};
use fs_calendars_testdata::draft::{
    AppointmentDraft, AppointmentPatch, Appointments, CalendarSource, Error, FetchState,
    Recurrence, Refresh, Restorable, SeriesScope, Trash,
};
use fs_calendars_testdata::ids::{object_id, person};
use fs_calendars_testdata::sample::{at, timed};
use fs_calendars_testdata::sample_cals::{ACCOUNT, COLLECTION, FEED, IN_TRASH, SUBSCRIBED};
use fs_calendars_testdata::zone::reference;

#[tokio::test]
async fn the_sample_has_a_subscription_an_account_and_a_calendar_in_the_trash() {
    let (_, f) = loaded();
    let all = f.calendars(Trash::Include).await.unwrap();
    assert_eq!(all.len(), 5);
    let only = f.calendars(Trash::Only).await.unwrap();
    assert_eq!(only.len(), 1);
    assert_eq!(only[0].id, object_id(IN_TRASH));
    assert!(matches!(only[0].trash, Some(Restorable::Until(_))));
    assert_eq!(f.calendars(Trash::Exclude).await.unwrap().len(), 4);
    let sub = all.iter().find(|c| c.id == object_id(SUBSCRIBED)).unwrap();
    assert!(
        matches!(&sub.source, CalendarSource::Subscribed(s) if s.url == Some(reference(FEED)) && matches!(s.state, FetchState::Fresh { .. }))
    );
    let acc = all.iter().find(|c| c.id == object_id(ACCOUNT)).unwrap();
    assert!(
        matches!(&acc.source, CalendarSource::Account { collection, .. } if *collection == Some(reference(COLLECTION)))
    );
    // Someone who may not change the calendar sees neither address.
    f.set_viewer(person(2));
    let seen = f.calendars(Trash::Exclude).await.unwrap();
    assert!(
        seen.iter()
            .any(|c| matches!(&c.source, CalendarSource::Subscribed(s) if s.url.is_none()))
    );
    assert!(seen.iter().any(|c| matches!(
        &c.source,
        CalendarSource::Account {
            collection: None,
            ..
        }
    )));
}

#[tokio::test]
async fn the_calendar_in_the_trash_of_the_sample_can_come_back() {
    let (s, f) = loaded();
    let kick = s.get("trash-kickoff");
    assert!(f.get(kick.id).await.unwrap().is_none());
    let r = f.restore_calendar(object_id(IN_TRASH)).await.unwrap();
    assert!(r.trash.is_none());
    assert!(f.get(kick.id).await.unwrap().is_some());
}

#[tokio::test]
async fn a_feed_whose_name_now_points_into_the_private_net_is_refused_at_every_fetch() {
    let f = fs_calendars_testdata::FakeAppointments::new();
    let url = reference("https://feed.example.org/a.ics");
    let c = f
        .subscribe_calendar("Feed", None, &url, Refresh::Manual)
        .await
        .unwrap();
    f.set_feed("https://feed.example.org/a.ics", &[("u1", "Eins")]);
    assert!(f.refresh_calendar(c.id).await.is_ok());
    f.set_dns("feed.example.org", "10.0.0.7".parse().unwrap());
    let r = f.refresh_calendar(c.id).await;
    assert!(
        matches!(r, Err(Error::Invalid(p)) if p[0].field == "url" && p[0].reason == "url.private")
    );
}

#[tokio::test]
async fn an_original_rule_shows_its_first_occurrence_and_cannot_be_made_or_changed() {
    let (s, f) = loaded();
    let a = s.get("original-series");
    let w = window(&f, utc(2027, 5, 1, 0, 0), utc(2029, 6, 1, 0, 0)).await;
    assert_eq!(
        w.instances.iter().filter(|i| i.appointment == a.id).count(),
        1
    );
    let title = AppointmentPatch {
        title: Some("Neu".to_owned()),
        ..AppointmentPatch::default()
    };
    let kept = f
        .update(a.id, a.revision, title, SeriesScope::Whole)
        .await
        .unwrap();
    assert_eq!(
        kept.recurrence, a.recurrence,
        "a change of other fields keeps the original text"
    );
    let changed = AppointmentPatch {
        recurrence: Some(vec![Recurrence::Original(
            "FREQ=YEARLY;BYWEEKNO=21".to_owned(),
        )]),
        ..AppointmentPatch::default()
    };
    let r = f
        .update(a.id, kept.revision, changed.clone(), SeriesScope::Whole)
        .await;
    assert!(
        matches!(r, Err(Error::Unsupported { capability, .. }) if capability == "event.update")
    );
    let draft = AppointmentDraft {
        calendar: object_id(1),
        uid: None,
        title: "Neu".to_owned(),
        when: timed(at(2027, 6, 1, 9, 0), "Europe/Berlin", 30),
        rest: changed,
    };
    let r = f.create(draft).await;
    assert!(
        matches!(r, Err(Error::Unsupported { capability, .. }) if capability == "event.create")
    );
}

#[tokio::test]
async fn an_exception_may_not_carry_a_recurrence() {
    let (s, f) = loaded();
    let a = s.get("weekly");
    let with_rule = AppointmentPatch {
        recurrence: Some(Vec::new()),
        ..AppointmentPatch::default()
    };
    let scope = SeriesScope::This {
        recurrence_id: at(2026, 11, 16, 10, 0),
    };
    let r = f.update(a.id, a.revision, with_rule, scope).await;
    assert!(matches!(r, Err(Error::Invalid(p)) if p[0].field == "recurrence"));
}

#[tokio::test]
async fn never_automatically_with_a_retention_that_has_an_end_is_held_until_then() {
    let (s, f) = loaded();
    f.set_trash_period(None);
    let a = s.get("single-berlin");
    let d = f
        .delete(a.id, a.revision, SeriesScope::Whole)
        .await
        .unwrap();
    assert_eq!(d.restorable, Restorable::Manual);
    let end = fs_calendars_testdata::draft::Utc::from_millis(f.now().millis() + 5 * 86_400_000);
    f.hold(a.id, Some(end));
    let q = fs_calendars_testdata::draft::AppointmentQuery {
        trash: Trash::Only,
        ..Default::default()
    };
    let hit = f
        .list(&q, None, 100)
        .await
        .unwrap()
        .items
        .into_iter()
        .find(|x| x.id == a.id)
        .unwrap();
    assert_eq!(hit.trash, Some(Restorable::Held(Some(end))));
}
