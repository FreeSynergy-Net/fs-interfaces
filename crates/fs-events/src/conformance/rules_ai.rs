//! Rules for an asking AI (ADR 0056): never owner, never purge/delete/subscribe, sees only what it was invited to.
//!
//! Pattern: Template method (rules as plain checks collected in the report).

use super::cx::{Cx, forbidden, not_found_of};
use super::fixture::Asker;
use crate::draft::{Attendee, CalendarPatch, FreeBusyAnswer, Refresh, SeriesScope, Trash, Utc};
use crate::ids::person;
use crate::zone::reference;

pub(super) async fn ai(cx: &mut Cx<'_>) {
    let dir = cx.dir;
    let Some(ai) = cx.fx.ai_person() else {
        cx.skip("ai: the fixture has no AI account");
        return;
    };
    // The owner makes the world: a calendar the AI is invited to, one it is not.
    let (Some(shown), Some(hidden)) = (
        cx.calendar("ai-shown").await,
        cx.calendar("ai-hidden").await,
    ) else {
        return;
    };
    let (Some(s_a), Some(h_a), Some(trashed)) = (
        cx.appointment(shown.id, "shown", 2).await,
        cx.appointment(hidden.id, "hidden", 3).await,
        cx.appointment(shown.id, "trashed", 4).await,
    ) else {
        return;
    };
    let _ = dir
        .delete(trashed.id, trashed.revision, SeriesScope::Whole)
        .await;
    if !(cx.fx.invite_ai(shown.id).await && cx.fx.switch_asker(Asker::Owner).await) {
        cx.skip("ai: the fixture cannot invite the AI");
        return;
    }
    let as_owner = CalendarPatch {
        owner: Some(ai.clone()),
        ..CalendarPatch::default()
    };
    let r = dir
        .update_calendar(shown.id, shown.revision, as_owner)
        .await;
    cx.fails(
        &r,
        "update_calendar: an AI as new owner is Forbidden(calendar.update)",
        forbidden("calendar.update"),
    );
    let r = dir
        .create_calendar("Zzconf ai-owned", None, None, Some(ai.clone()))
        .await;
    cx.fails(
        &r,
        "create_calendar: an AI as owner is Forbidden(calendar.create)",
        forbidden("calendar.create"),
    );
    if !cx.fx.switch_asker(Asker::Ai).await {
        cx.skip("ai: the fixture cannot switch the asker to the AI");
        return;
    }
    let r = dir.create_calendar("Zzconf by ai", None, None, None).await;
    cx.fails(
        &r,
        "create_calendar: an asking AI would own it: Forbidden(calendar.create)",
        forbidden("calendar.create"),
    );
    let patch = CalendarPatch {
        name: Some("Zzconf by ai".to_owned()),
        ..CalendarPatch::default()
    };
    let r = dir.update_calendar(shown.id, shown.revision, patch).await;
    cx.fails(
        &r,
        "update_calendar: an AI may not change it: Forbidden(calendar.update)",
        forbidden("calendar.update"),
    );
    let r = dir.delete_calendar(shown.id, shown.revision).await;
    cx.fails(
        &r,
        "delete_calendar: an AI never deletes a calendar: Forbidden(calendar.delete)",
        forbidden("calendar.delete"),
    );
    let r = dir
        .subscribe_calendar(
            "Zzconf by ai",
            None,
            &reference("https://kalender.example.org/zzconf-ai.ics"),
            Refresh::Manual,
        )
        .await;
    cx.fails(
        &r,
        "subscribe_calendar: an AI never subscribes: Forbidden(calendar.subscribe)",
        forbidden("calendar.subscribe"),
    );
    let r = dir.purge(trashed.id).await;
    cx.fails(
        &r,
        "purge: an AI never purges: Forbidden(trash.purge)",
        forbidden("trash.purge"),
    );
    cx.ensure(
        cx.fresh(s_a.id).await.is_some(),
        "an AI sees an appointment of the calendar it was invited to",
    );
    // What it may not see answers like an unknown id: NotFound, never Forbidden (that would tell it exists).
    let seen = dir.calendars(Trash::Include).await.unwrap_or_default();
    cx.ensure(
        seen.iter().any(|c| c.id == shown.id) && seen.iter().all(|c| c.id != hidden.id),
        "calendars: an AI sees only the calendars it was invited to",
    );
    cx.ensure(
        cx.fresh(h_a.id).await.is_none(),
        "get: an appointment of a hidden calendar is None for an AI",
    );
    cx.ensure(
        cx.all(&[], Trash::Include)
            .await
            .iter()
            .all(|s| s.id != h_a.id),
        "list: nor is it listed",
    );
    let hp = CalendarPatch {
        name: Some("x".to_owned()),
        ..CalendarPatch::default()
    };
    let r = dir.update_calendar(hidden.id, hidden.revision, hp).await;
    cx.fails(
        &r,
        "update_calendar: a hidden calendar is NotFound for an AI (not Forbidden)",
        not_found_of(hidden.id),
    );
    let r = dir.delete_calendar(hidden.id, hidden.revision).await;
    cx.fails(
        &r,
        "delete_calendar: a hidden calendar is NotFound for an AI (not Forbidden)",
        not_found_of(hidden.id),
    );
    let r = dir.history(h_a.id, None, 10).await;
    cx.fails(
        &r,
        "history: a hidden appointment is NotFound for an AI",
        not_found_of(h_a.id),
    );
    // free_busy: only the whole call fails, never one person (not even for an AI).
    let from = Utc::from_millis(4_000_000_000_000);
    let to = Utc::from_millis(from.millis() + 7 * 86_400_000);
    let who = [
        Attendee::Actor(person(5001)),
        Attendee::Actor(shown.owner.clone()),
    ];
    let r = dir.free_busy(&who, from, to, &cx.berlin).await;
    let ok = r
        .as_ref()
        .is_ok_and(|a| a.len() == 2 && a[0].answer == FreeBusyAnswer::NotShared);
    cx.ensure(
        ok,
        "free_busy: for an AI a stranger is NotShared, never NotFound",
    );
    cx.fx.switch_asker(Asker::Owner).await;
}
