//! Failing, slow, empty, people, and the sample itself.
#![allow(
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    clippy::manual_assert_eq
)]

mod common;

use std::collections::HashSet;
use std::time::{Duration, Instant};

use common::{loaded, utc};
use fs_calendars_testdata::draft::{
    AppointmentPatch, Appointments, BasicValue, CAPABILITIES, ContactAddress, Error, PersonArt,
    Revision, SeriesScope, Trash,
};
use fs_calendars_testdata::ids::{object_id, person};
use fs_calendars_testdata::people::{Directory, ProposeEntry};
use fs_calendars_testdata::{DECISIONS, FailingAppointments, SlowAppointments, sample};

#[tokio::test]
async fn every_method_of_the_failing_provider_is_unavailable() {
    let f = FailingAppointments::new("down");
    let id = object_id(1);
    assert!(matches!(f.calendars(Trash::Exclude).await, Err(Error::Unavailable(r)) if r == "down"));
    assert!(matches!(f.get(id).await, Err(Error::Unavailable(_))));
    assert!(matches!(
        f.window(
            utc(2026, 1, 1, 0, 0),
            utc(2026, 1, 2, 0, 0),
            &common::utc_zone(),
            &[]
        )
        .await,
        Err(Error::Unavailable(_))
    ));
    assert!(matches!(
        f.update(
            id,
            Revision(1),
            AppointmentPatch::default(),
            SeriesScope::Whole
        )
        .await,
        Err(Error::Unavailable(_))
    ));
    assert!(matches!(
        f.free_busy(
            &[],
            utc(2026, 1, 1, 0, 0),
            utc(2026, 1, 2, 0, 0),
            &common::utc_zone(),
        )
        .await,
        Err(Error::Unavailable(_))
    ));
    assert!(matches!(
        f.changes(None, 1).await,
        Err(Error::Unavailable(_))
    ));
    assert!(
        f.capabilities().0.iter().all(|(_, on)| *on),
        "it claims to be able"
    );
}

#[tokio::test]
async fn the_slow_provider_waits_and_then_answers_like_the_inner_one() {
    let (_, fake) = loaded();
    let slow = SlowAppointments::new(fake, Duration::from_millis(120));
    let t = Instant::now();
    let w = slow
        .window(
            utc(2026, 11, 17, 0, 0),
            utc(2026, 11, 18, 0, 0),
            &common::utc_zone(),
            &[],
        )
        .await
        .unwrap();
    assert!(t.elapsed() >= Duration::from_millis(120));
    assert!(!w.instances.is_empty());
}

#[tokio::test]
async fn the_fake_offers_all_capabilities() {
    let (_, f) = loaded();
    let caps = f.capabilities().0;
    assert_eq!(caps.len(), CAPABILITIES.len());
    assert!(caps.iter().all(|(_, on)| *on));
    assert_eq!(
        f.create_calendar("Neu", None, None, None)
            .await
            .unwrap()
            .name,
        "Neu"
    );
    assert!(f.create_calendar(" ", None, None, None).await.is_err());
}

#[test]
fn the_sample_is_complete_and_has_no_names_in_appointments() {
    let s = sample();
    assert_eq!(s.appointments.len(), 50);
    let uids: HashSet<_> = s.appointments.iter().map(|(_, a)| a.uid.clone()).collect();
    let ids: HashSet<_> = s.appointments.iter().map(|(_, a)| a.id).collect();
    assert_eq!((uids.len(), ids.len()), (50, 50));
    let customs = std::iter::once(("/Custom/Aussenstelle".to_owned(), 19_800)).collect();
    for (slug, a) in &s.appointments {
        fs_calendars_testdata::check::appointment(a, &customs)
            .unwrap_or_else(|e| panic!("{slug}: {e:?}"));
        assert!(
            a.participants
                .iter()
                .all(|p| fs_calendars_testdata::ids::actor(p)
                    .is_none_or(|x| x.id.canonical().starts_with("urn:uuid:"))),
            "{slug}: people are references"
        );
    }
    assert!(s.get("floating-imported").when.clone() != s.get("single-berlin").when);
    assert_eq!(s.calendars.len(), 5);
    assert_eq!(DECISIONS.len(), 13);
}

fn address(text: &str) -> ContactAddress {
    ContactAddress::parse(text).unwrap()
}

#[tokio::test]
async fn the_people_directory_finds_and_proposes_in_one_batch() {
    let dir = sample().directory();
    let hits = dir
        .find_by_address(&address("mailto:MAREK.LINDE@example.org"))
        .await
        .unwrap();
    assert_eq!(hits, vec![person(2)]);
    assert_eq!(
        dir.display(&hits[0]).await.unwrap().unwrap().0,
        "Marek Linde"
    );
    assert!(
        dir.find_by_address(&address("mailto:new@example.org"))
            .await
            .unwrap()
            .is_empty()
    );
    let origin = fs_calendars_testdata::zone::reference(
        "urn:sha256:0000000000000000000000000000000000000000000000000000000000000002",
    );
    let batch = dir.open_batch(&origin).await.unwrap();
    let entry = ProposeEntry {
        display_hint: "Neue Person".to_owned(),
        address: Some(address("mailto:new@example.org")),
        origin: Some(origin.clone()),
    };
    let p = dir.propose(&entry, Some(batch)).await.unwrap();
    assert_eq!(p.art, PersonArt::Profile);
    let again = ProposeEntry {
        display_hint: "andere".to_owned(),
        ..entry
    };
    assert_eq!(
        dir.propose(&again, Some(batch)).await.unwrap(),
        p,
        "no second entry for one address"
    );
    let made = dir.proposed();
    assert_eq!(made.len(), 1);
    assert_eq!(
        (made[0].batch, made[0].origin.clone()),
        (Some(batch), Some(origin))
    );
}

#[tokio::test]
async fn an_asking_ai_gets_no_address_oracle_from_the_directory() {
    let dir = sample().directory().with_ai();
    assert!(matches!(
        dir.find_by_address(&address("mailto:marek.linde@example.org")).await,
        Err(Error::Forbidden { capability }) if capability == "person.find"
    ));
    let entry = ProposeEntry {
        display_hint: "X".to_owned(),
        address: Some(address("mailto:x@example.org")),
        origin: None,
    };
    assert!(matches!(
        dir.propose(&entry, None).await,
        Err(Error::Forbidden { capability }) if capability == "person.propose"
    ));
}
