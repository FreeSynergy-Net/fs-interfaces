//! The series question: this, this and following, whole (dossier A9, A10).
#![allow(
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    clippy::manual_assert_eq
)]

mod common;

use common::{loaded, utc, window};
use fs_calendars_testdata::draft::{
    Appointment, AppointmentPatch, Appointments, Error, OverrideChange, Recurrence, Relation,
    Restorable, RuleEnd, SeriesScope,
};
use fs_calendars_testdata::sample::at;

#[tokio::test]
async fn delete_this_makes_an_excluded_override() {
    let (s, f) = loaded();
    let a = s.get("weekly");
    let r = f
        .delete(
            a.id,
            a.revision,
            SeriesScope::This {
                recurrence_id: at(2026, 11, 16, 10, 0),
            },
        )
        .await
        .unwrap();
    assert!(
        matches!(r.restorable, Restorable::Until(_)),
        "a partial delete can be undone with restore"
    );
    let now = f.get(a.id).await.unwrap().unwrap();
    assert!(now.overrides.iter().any(
        |o| o.recurrence_id == at(2026, 11, 16, 10, 0) && o.change == OverrideChange::Excluded
    ));
    let w = window(&f, utc(2026, 11, 16, 0, 0), utc(2026, 11, 17, 0, 0)).await;
    assert!(w.instances.iter().all(|i| i.appointment != a.id));
}

#[tokio::test]
async fn delete_this_and_following_cuts_the_series() {
    let (s, f) = loaded();
    let a = s.get("weekly");
    f.delete(
        a.id,
        a.revision,
        SeriesScope::ThisAndFollowing {
            recurrence_id: at(2026, 11, 25, 10, 0),
        },
    )
    .await
    .unwrap();
    let now = f.get(a.id).await.unwrap().unwrap();
    assert!(
        matches!(&now.recurrence[0], Recurrence::Rule(r) if matches!(r.end, RuleEnd::Until(_)))
    );
    let w = window(&f, utc(2026, 11, 1, 0, 0), utc(2027, 6, 1, 0, 0)).await;
    let n = w.instances.iter().filter(|i| i.appointment == a.id).count();
    assert_eq!(
        n, 6,
        "Nov 2, 4, 9, 11, 16 and the moved 18th; the 23rd is cancelled, the 25th on is cut"
    );
}

#[tokio::test]
async fn change_this_keeps_the_rest_and_a_second_change_merges() {
    let (s, f) = loaded();
    let a = s.get("weekly");
    let p1 = AppointmentPatch {
        title: Some("Nur heute".to_owned()),
        ..AppointmentPatch::default()
    };
    let b = f
        .update(
            a.id,
            a.revision,
            p1,
            SeriesScope::This {
                recurrence_id: at(2026, 11, 16, 10, 0),
            },
        )
        .await
        .unwrap();
    let p2 = AppointmentPatch {
        priority: Some(3),
        ..AppointmentPatch::default()
    };
    let c = f
        .update(
            a.id,
            b.revision,
            p2,
            SeriesScope::This {
                recurrence_id: at(2026, 11, 16, 10, 0),
            },
        )
        .await
        .unwrap();
    let o = c
        .overrides
        .iter()
        .find(|o| o.recurrence_id == at(2026, 11, 16, 10, 0))
        .unwrap();
    let OverrideChange::Changed(p) = &o.change else {
        panic!("expected a change")
    };
    assert_eq!(
        (p.title.as_deref(), p.priority),
        (Some("Nur heute"), Some(3))
    );
    assert_eq!(c.title, "Teamrunde", "the master is unchanged");
    let w = window(&f, utc(2026, 11, 16, 0, 0), utc(2026, 11, 17, 0, 0)).await;
    assert_eq!(
        w.instances
            .iter()
            .find(|i| i.appointment == a.id)
            .unwrap()
            .title,
        "Nur heute"
    );
}

#[tokio::test]
async fn a_scope_that_is_not_an_occurrence_is_invalid() {
    let (s, f) = loaded();
    let a = s.get("weekly");
    let tuesday = SeriesScope::This {
        recurrence_id: at(2026, 11, 17, 10, 0),
    };
    assert!(matches!(
        f.delete(a.id, a.revision, tuesday).await,
        Err(Error::Invalid(_))
    ));
    let single = s.get("single-berlin");
    let r = f
        .update(
            single.id,
            single.revision,
            AppointmentPatch::default(),
            SeriesScope::This {
                recurrence_id: single_rid(),
            },
        )
        .await;
    assert!(matches!(r, Err(Error::Invalid(p)) if p[0].field == "scope"));
}

fn single_rid() -> fs_calendars_testdata::draft::LocalDateTime {
    at(2026, 11, 17, 9, 0)
}

#[tokio::test]
async fn this_and_following_splits_the_series() {
    let (s, f) = loaded();
    let a = s.get("weekly");
    let patch = AppointmentPatch {
        title: Some("Neue Runde".to_owned()),
        ..AppointmentPatch::default()
    };
    let tail: Appointment = f
        .update(
            a.id,
            a.revision,
            patch,
            SeriesScope::ThisAndFollowing {
                recurrence_id: at(2026, 11, 25, 10, 0),
            },
        )
        .await
        .unwrap();
    assert_ne!(tail.uid, a.uid);
    assert_eq!(tail.title, "Neue Runde");
    assert_eq!(
        tail.when,
        fs_calendars_testdata::sample::timed(at(2026, 11, 25, 10, 0), "Europe/Berlin", 30)
    );
    assert!(
        tail.related
            .iter()
            .any(|r| r.relation == Relation::First && r.uid == a.uid)
    );
    let head = f.get(a.id).await.unwrap().unwrap();
    assert!(
        head.related
            .iter()
            .any(|r| r.relation == Relation::Next && r.uid == tail.uid)
    );
    // 20 occurrences in all; 7 lie before the 25th (Nov 2, 4, 9, 11, 16, 18, 23), so the tail keeps 13.
    assert!(matches!(&tail.recurrence[0], Recurrence::Rule(r) if r.end == RuleEnd::Count(13)));
    let w = window(&f, utc(2026, 11, 1, 0, 0), utc(2027, 6, 1, 0, 0)).await;
    let tail_n = w
        .instances
        .iter()
        .filter(|i| i.appointment == tail.id)
        .count();
    let head_n = w.instances.iter().filter(|i| i.appointment == a.id).count();
    assert_eq!(
        (head_n, tail_n),
        (6, 13),
        "head: 7 minus the cancelled 23rd; tail: 13"
    );
}

#[tokio::test]
async fn this_and_following_on_the_first_occurrence_is_the_whole_series() {
    let (s, f) = loaded();
    let a = s.get("weekly");
    let patch = AppointmentPatch {
        title: Some("Alle".to_owned()),
        ..AppointmentPatch::default()
    };
    let r = f
        .update(
            a.id,
            a.revision,
            patch,
            SeriesScope::ThisAndFollowing {
                recurrence_id: at(2026, 11, 2, 10, 0),
            },
        )
        .await
        .unwrap();
    assert_eq!(r.id, a.id);
    assert_eq!(r.title, "Alle");
}
