//! Inviting and answering (A5, A11).
#![allow(
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    clippy::manual_assert_eq
)]

mod common;

use common::{loaded, utc, window};
use fs_calendars_testdata::draft::{
    Answer, Appointments, Attendee, Error, HistoryKind, Invitee, ParticipationStatus, Roles,
    SeriesScope, Value,
};
use fs_calendars_testdata::ids::person;
use fs_calendars_testdata::sample::at;

fn invitee(n: u64) -> Invitee {
    Invitee {
        who: Attendee::Actor(person(n)),
        roles: Roles {
            attendee: true,
            ..Roles::default()
        },
        expect_reply: true,
    }
}

#[tokio::test]
async fn invite_adds_people_by_reference_only_and_makes_the_viewer_the_owner() {
    let (s, f) = loaded();
    let a = s.get("single-berlin");
    let b = f
        .invite(
            a.id,
            a.revision,
            &[invitee(2), invitee(2), invitee(3)],
            SeriesScope::Whole,
        )
        .await
        .unwrap();
    assert_eq!(b.participants.len(), 3, "owner, Ann once, Ben");
    assert!(
        b.participants[0].roles.owner && b.participants[0].status == ParticipationStatus::Accepted
    );
    assert_eq!(b.participants[1].who, Attendee::Actor(person(2)));
    assert_eq!(b.participants[1].status, ParticipationStatus::NeedsAction);
    let h = f.history(a.id, None, 10).await.unwrap();
    assert_eq!(h.items[0].kind, HistoryKind::Invite);
    assert_eq!(
        h.items[0]
            .changes
            .iter()
            .filter(|c| c.field == "participant")
            .count(),
        2
    );
}

#[tokio::test]
async fn respond_changes_one_status_and_logs_old_and_new() {
    let (s, f) = loaded();
    let a = s.get("invitation");
    let b = f
        .respond(
            a.id,
            a.revision,
            &person(4),
            Answer::Tentative,
            Some("vielleicht"),
            None,
            SeriesScope::Whole,
        )
        .await
        .unwrap();
    let clara = b
        .participants
        .iter()
        .find(|p| p.who == Attendee::Actor(person(4)))
        .unwrap();
    assert_eq!(
        (clara.status, clara.comment.as_deref()),
        (ParticipationStatus::Tentative, Some("vielleicht"))
    );
    let h = f.history(a.id, None, 10).await.unwrap();
    let e = &h.items[0];
    assert_eq!(e.kind, HistoryKind::TentativeAccept);
    let c = e
        .changes
        .iter()
        .find(|c| c.field == "participant.status")
        .unwrap();
    assert_eq!(
        (c.row_key.as_deref(), c.old.clone(), c.new.clone()),
        (
            Some("p4"),
            Some(Value::Text("NeedsAction".into())),
            Some(Value::Text("Tentative".into()))
        )
    );
}

#[tokio::test]
async fn a_proposal_is_logged_and_a_stranger_cannot_answer() {
    let (s, f) = loaded();
    let a = s.get("invitation");
    let when = fs_calendars_testdata::sample::timed(at(2026, 11, 19, 16, 0), "Europe/Berlin", 60);
    f.respond(
        a.id,
        a.revision,
        &person(2),
        Answer::Decline,
        None,
        Some(when),
        SeriesScope::Whole,
    )
    .await
    .unwrap();
    let h = f.history(a.id, None, 10).await.unwrap();
    assert!(h.items[0].changes.iter().any(
        |c| c.field == "proposal.start" && c.new == Some(Value::Local(at(2026, 11, 19, 16, 0)))
    ));
    assert_eq!(h.items[0].kind, HistoryKind::Reject);
    assert!(
        matches!(f.respond(a.id, f.get(a.id).await.unwrap().unwrap().revision, &person(9), Answer::Accept, None, None, SeriesScope::Whole).await, Err(Error::Invalid(p)) if p[0].field == "who")
    );
}

#[tokio::test]
async fn my_status_shows_in_the_window_and_answers_for_one_occurrence_only() {
    let (s, f) = loaded();
    let a = s.get("invitation");
    let w = window(&f, utc(2026, 11, 19, 0, 0), utc(2026, 11, 20, 0, 0)).await;
    let i = w.instances.iter().find(|i| i.appointment == a.id).unwrap();
    assert_eq!(
        (i.my_status, i.participant_count),
        (Some(ParticipationStatus::Accepted), 5)
    );
    let series = s.get("weekly");
    f.invite(
        series.id,
        series.revision,
        &[invitee(2)],
        SeriesScope::This {
            recurrence_id: at(2026, 11, 16, 10, 0),
        },
    )
    .await
    .unwrap();
    let w = window(&f, utc(2026, 11, 16, 0, 0), utc(2026, 11, 30, 0, 0)).await;
    let counts: Vec<u32> = w
        .instances
        .iter()
        .filter(|i| i.appointment == series.id)
        .map(|i| i.participant_count)
        .collect();
    assert_eq!(counts, vec![2, 0, 0], "only the 16th has guests");
}
