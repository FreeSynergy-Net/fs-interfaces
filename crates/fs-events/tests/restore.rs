//! Undo of a partial delete (`restore` with the scope of the delete) and `respond` with `expected`.
#![allow(clippy::many_single_char_names)]

mod common;

use common::{loaded, utc, window};
use fs_calendars_testdata::draft::{Answer, Appointments, Error, Restorable, SeriesScope};
use fs_calendars_testdata::ids::person;
use fs_calendars_testdata::sample::at;

fn count(
    w: &fs_calendars_testdata::draft::Window,
    id: fs_calendars_testdata::draft::ObjectId,
) -> usize {
    w.instances.iter().filter(|i| i.appointment == id).count()
}

#[tokio::test]
async fn restore_this_takes_the_excluded_override_back() {
    let (s, f) = loaded();
    let a = s.get("weekly");
    let (from, to) = (utc(2026, 11, 1, 0, 0), utc(2027, 6, 1, 0, 0));
    let before = count(&window(&f, from, to).await, a.id);
    let scope = SeriesScope::This {
        recurrence_id: at(2026, 11, 16, 10, 0),
    };
    let d = f.delete(a.id, a.revision, scope).await.unwrap();
    assert!(matches!(d.restorable, Restorable::Until(_)));
    assert_eq!(count(&window(&f, from, to).await, a.id), before - 1);
    f.restore(a.id, scope).await.unwrap();
    assert_eq!(count(&window(&f, from, to).await, a.id), before);
    assert!(matches!(
        f.restore(a.id, scope).await,
        Err(Error::Invalid(_))
    ));
}

#[tokio::test]
async fn restore_this_and_following_gives_the_series_back() {
    let (s, f) = loaded();
    let a = s.get("weekly");
    let (from, to) = (utc(2026, 11, 1, 0, 0), utc(2027, 6, 1, 0, 0));
    let before = count(&window(&f, from, to).await, a.id);
    let scope = SeriesScope::ThisAndFollowing {
        recurrence_id: at(2026, 11, 25, 10, 0),
    };
    f.delete(a.id, a.revision, scope).await.unwrap();
    assert!(count(&window(&f, from, to).await, a.id) < before);
    f.restore(a.id, scope).await.unwrap();
    let now = f.get(a.id).await.unwrap().unwrap();
    assert_eq!(now.recurrence, a.recurrence);
    assert_eq!(count(&window(&f, from, to).await, a.id), before);
}

#[tokio::test]
async fn restore_after_the_period_or_without_a_delete_is_refused() {
    let (s, f) = loaded();
    let a = s.get("weekly");
    let scope = SeriesScope::This {
        recurrence_id: at(2026, 11, 16, 10, 0),
    };
    assert!(matches!(
        f.restore(a.id, scope).await,
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        f.restore(a.id, SeriesScope::Whole).await,
        Err(Error::Invalid(_))
    ));
    f.delete(a.id, a.revision, scope).await.unwrap();
    f.set_now(utc(2027, 3, 1, 0, 0));
    assert!(matches!(
        f.restore(a.id, scope).await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn respond_compares_against_expected() {
    let (s, f) = loaded();
    let a = s.get("invitation");
    let stale = fs_calendars_testdata::draft::Revision(a.revision.0 + 5);
    let r = f
        .respond(
            a.id,
            stale,
            &person(4),
            Answer::Accept,
            None,
            None,
            SeriesScope::Whole,
        )
        .await;
    assert!(matches!(r, Err(Error::Changed { current }) if current == a.revision));
    let ok = f
        .respond(
            a.id,
            a.revision,
            &person(4),
            Answer::Accept,
            None,
            None,
            SeriesScope::Whole,
        )
        .await;
    assert!(ok.is_ok());
}

#[tokio::test]
async fn restore_of_a_deleted_changed_occurrence_gives_the_change_back() {
    let (s, f) = loaded();
    let a = s.get("weekly");
    let scope = SeriesScope::This {
        recurrence_id: at(2026, 11, 18, 10, 0),
    };
    f.delete(a.id, a.revision, scope).await.unwrap();
    let back = f.restore(a.id, scope).await.unwrap();
    let was = a
        .overrides
        .iter()
        .find(|o| o.recurrence_id == at(2026, 11, 18, 10, 0));
    assert!(
        was.is_some() && back.overrides.iter().any(|o| Some(o) == was),
        "the moved occurrence is back as it was"
    );
}

#[tokio::test]
async fn restore_this_and_following_reports_changed_after_a_later_change_and_the_period_is_a_closed_bound()
 {
    let (s, f) = loaded();
    let a = s.get("weekly");
    let scope = SeriesScope::ThisAndFollowing {
        recurrence_id: at(2026, 11, 25, 10, 0),
    };
    f.delete(a.id, a.revision, scope).await.unwrap();
    let cur = f.get(a.id).await.unwrap().unwrap();
    let patch = fs_calendars_testdata::draft::AppointmentPatch {
        title: Some("Neu".into()),
        ..fs_calendars_testdata::draft::AppointmentPatch::default()
    };
    let changed = f
        .update(a.id, cur.revision, patch, SeriesScope::Whole)
        .await
        .unwrap();
    let r = f.restore(a.id, scope).await;
    assert!(
        matches!(r, Err(Error::Changed { current }) if current == changed.revision),
        "{r:?}"
    );
    // The bound: exactly at `deleted + period` the restore is over (Local: `purge_after <= now`).
    let at_ms = |delta: i64| {
        let (s2, f2) = loaded();
        let b = s2.get("weekly").clone();
        let this = SeriesScope::This {
            recurrence_id: at(2026, 11, 16, 10, 0),
        };
        (f2, b, this, delta)
    };
    for (delta, ok) in [(-1_i64, true), (0, false)] {
        let (f2, b, this, d) = at_ms(delta);
        let Restorable::Until(until) = f2.delete(b.id, b.revision, this).await.unwrap().restorable
        else {
            panic!("a period is set")
        };
        f2.set_now(fs_calendars_testdata::draft::Utc::from_millis(
            until.millis() + d,
        ));
        let r = f2.restore(b.id, this).await;
        assert_eq!(r.is_ok(), ok, "delta {delta}: {r:?}");
    }
}
