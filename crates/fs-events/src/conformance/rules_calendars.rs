//! Rules for calendars: create, change, owner, and the trash of a whole calendar with everything in it.
//!
//! Pattern: Template method (rules as plain checks collected in the report).

use super::cx::{Cx, forbidden, invalid_at, is_changed, is_invalid, is_not_found, not_found_of};
use crate::draft::{
    Answer, AppointmentPatch, CalendarPatch, CalendarSource, PersonArt, PersonRef, Refresh,
    Restorable, Revision, SeriesScope, Trash,
};
use crate::ids::person_iri;

pub(super) async fn manage(cx: &mut Cx<'_>) {
    let dir = cx.dir;
    let Some(c) = cx.calendar("manage").await else {
        return;
    };
    cx.ensure(
        c.source == CalendarSource::Own && c.trash.is_none() && c.revision == Revision(1),
        "create_calendar: an own, living calendar",
    );
    let bad_zone = crate::zone::iana("Mars/Olympus");
    let r = dir
        .create_calendar("Zzconf zone", None, Some(bad_zone), None)
        .await;
    cx.fails(
        &r,
        "create_calendar: an unknown zone is Invalid (zone.bad)",
        invalid_at("default_zone", Some("zone.bad")),
    );
    let r = dir.create_calendar("  ", None, None, None).await;
    cx.fails(
        &r,
        "create_calendar: an empty name is Invalid",
        invalid_at("name", None),
    );
    let patch = |name: &str| CalendarPatch {
        name: Some(name.to_owned()),
        ..CalendarPatch::default()
    };
    let stale = dir
        .update_calendar(c.id, Revision(c.revision.0 + 5), patch("x"))
        .await;
    cx.fails(
        &stale,
        "update_calendar: a stale revision is Changed",
        is_changed,
    );
    let Ok(c2) = dir
        .update_calendar(c.id, c.revision, patch("Zzconf manage 2"))
        .await
    else {
        cx.ensure(false, "update_calendar: answers");
        return;
    };
    cx.ensure(
        c2.name == "Zzconf manage 2" && c2.revision.0 > c.revision.0,
        "update_calendar: changes the name and the revision",
    );
    let r = dir.update_calendar(c.id, c2.revision, patch(" ")).await;
    cx.fails(
        &r,
        "update_calendar: an empty name is Invalid",
        invalid_at("name", None),
    );
    let own_url = CalendarPatch {
        refresh: Some(Refresh::Manual),
        ..CalendarPatch::default()
    };
    let r = dir.update_calendar(c.id, c2.revision, own_url).await;
    cx.fails(
        &r,
        "update_calendar: refresh on an own calendar is Invalid",
        invalid_at("refresh", None),
    );
    let profile = PersonRef {
        id: person_iri(777),
        art: PersonArt::Profile,
        abzug: None,
    };
    let to_profile = CalendarPatch {
        owner: Some(profile),
        ..CalendarPatch::default()
    };
    let r = dir.update_calendar(c.id, c2.revision, to_profile).await;
    cx.fails(
        &r,
        "update_calendar: a profile as owner is Invalid",
        invalid_at("owner", None),
    );
    let group = PersonRef {
        id: person_iri(778),
        art: PersonArt::Group,
        abzug: None,
    };
    let made = dir
        .create_calendar("Zzconf group", None, None, Some(group.clone()))
        .await;
    cx.ensure(
        made.as_ref().is_ok_and(|g| g.owner == group)
            || made
                .as_ref()
                .is_err_and(|e| forbidden("calendar.create")(e)),
        "create_calendar: a group may own (or the asker has no right for it: Forbidden)",
    );
}

/// Delete a calendar: it takes its appointments along; only `restore_calendar` and `purge` reach it.
pub(super) async fn cascade(cx: &mut Cx<'_>) {
    let dir = cx.dir;
    let Some(c) = cx.calendar("cascade").await else {
        return;
    };
    let (Some(a), Some(b), Some(early)) = (
        cx.appointment(c.id, "a", 2).await,
        cx.appointment(c.id, "b", 3).await,
        cx.appointment(c.id, "early", 4).await,
    ) else {
        return;
    };
    // One appointment goes to the trash on its own first; it must stay there when the calendar comes back.
    let Ok(early_gone) = dir
        .delete(early.id, early.revision, SeriesScope::Whole)
        .await
    else {
        cx.ensure(false, "delete: answers");
        return;
    };
    let stale = dir.delete_calendar(c.id, Revision(c.revision.0 + 5)).await;
    cx.fails(
        &stale,
        "delete_calendar: a stale revision is Changed",
        is_changed,
    );
    let Ok(d) = dir.delete_calendar(c.id, c.revision).await else {
        cx.ensure(false, "delete_calendar: answers");
        return;
    };
    cx.ensure(
        d.id == c.id && d.appointments == 2,
        "delete_calendar: takes the living appointments along (and says how many)",
    );
    cx.ensure(
        !matches!(d.restorable, Restorable::Held(_)),
        "delete_calendar: no retention, so not Held",
    );
    let living = dir.calendars(Trash::Exclude).await.unwrap_or_default();
    cx.ensure(
        living.iter().all(|x| x.id != c.id),
        "calendars(Exclude): the deleted one is gone",
    );
    let only = dir.calendars(Trash::Only).await.unwrap_or_default();
    cx.ensure(
        only.iter()
            .any(|x| x.id == c.id && x.trash == Some(d.restorable)),
        "calendars(Only): the deleted one, with the restorable of the delete",
    );
    let inc = cx.listed(c.id).await;
    cx.ensure(
        inc.is_some_and(|x| x.trash.is_some()),
        "calendars(Include): shows it with `trash`",
    );
    cx.ensure(
        cx.fresh(a.id).await.is_none() && cx.all(&[c.id], Trash::Exclude).await.is_empty(),
        "get/list(Exclude): its appointments are in the trash too",
    );
    let in_trash: Vec<_> = cx.all(&[c.id], Trash::Only).await;
    cx.ensure(
        in_trash.len() == 3
            && in_trash
                .iter()
                .any(|s| s.id == a.id && s.trash == Some(d.restorable)),
        "list(Only): all its appointments with the deadline of the calendar",
    );
    // Writing into a calendar in the trash is NotFound(calendar), like a task in a deleted list.
    let nf = not_found_of(c.id);
    let draft = crate::draft::AppointmentDraft {
        calendar: c.id,
        uid: None,
        title: "Zzconf late".to_owned(),
        when: Cx::when(5),
        rest: AppointmentPatch::default(),
    };
    let r = dir.create(draft).await;
    cx.fails(
        &r,
        "create in a calendar in the trash is NotFound(calendar)",
        &nf,
    );
    let patch = AppointmentPatch {
        title: Some("x".to_owned()),
        ..AppointmentPatch::default()
    };
    let r = dir
        .update(a.id, a.revision, patch, SeriesScope::Whole)
        .await;
    cx.fails(
        &r,
        "update of an appointment in a calendar in the trash is NotFound(calendar)",
        &nf,
    );
    let r = dir.delete(a.id, a.revision, SeriesScope::Whole).await;
    cx.fails(
        &r,
        "delete of an appointment in a calendar in the trash is NotFound(calendar)",
        &nf,
    );
    if cx.offers(&["invite.create", "invite.answer"]) {
        let r = dir.invite(a.id, a.revision, &[], SeriesScope::Whole).await;
        cx.fails(
            &r,
            "invite in a calendar in the trash is NotFound(calendar)",
            &nf,
        );
        let me = c.owner.clone();
        let r = dir
            .respond(
                a.id,
                a.revision,
                &me,
                Answer::Accept,
                None,
                None,
                SeriesScope::Whole,
            )
            .await;
        cx.fails(
            &r,
            "respond in a calendar in the trash is NotFound(calendar)",
            &nf,
        );
    }
    let r = dir.restore(a.id, SeriesScope::Whole).await;
    cx.fails(
        &r,
        "restore of an appointment in a calendar in the trash is Invalid(calendar)",
        invalid_at("calendar", None),
    );
    let nf = not_found_of(c.id);
    let patch = CalendarPatch {
        name: Some("Zzconf nope".to_owned()),
        ..CalendarPatch::default()
    };
    let r = dir.update_calendar(c.id, c.revision, patch).await;
    cx.fails(
        &r,
        "update_calendar on a calendar in the trash is NotFound(id)",
        &nf,
    );
    let r = dir.delete_calendar(c.id, c.revision).await;
    cx.fails(
        &r,
        "delete_calendar on a calendar in the trash is NotFound(id)",
        &nf,
    );
    if cx.offers(&["calendar.refresh"]) {
        let r = dir.refresh_calendar(c.id).await;
        cx.fails(
            &r,
            "refresh_calendar on a calendar in the trash is NotFound(id)",
            &nf,
        );
    }
    // Back: the calendar and what went with it; the one deleted before stays in the trash.
    let r = dir.restore_calendar(c.id).await;
    cx.ensure(
        r.as_ref().is_ok_and(|x| x.trash.is_none()),
        "restore_calendar: the calendar is living again",
    );
    cx.ensure(
        cx.fresh(a.id).await.is_some() && cx.fresh(b.id).await.is_some(),
        "restore_calendar: the appointments that went with it are back",
    );
    cx.ensure(
        cx.fresh(early.id).await.is_none()
            && cx
                .all(&[c.id], Trash::Only)
                .await
                .iter()
                .any(|s| s.id == early.id && s.trash == Some(early_gone.restorable)),
        "restore_calendar: one deleted before stays in the trash with its own deadline",
    );
    let r = dir.restore_calendar(c.id).await;
    cx.fails(
        &r,
        "restore_calendar: a living calendar is Invalid",
        is_invalid,
    );
    let r = dir.restore_calendar(crate::ids::object_id(u64::MAX)).await;
    cx.fails(
        &r,
        "restore_calendar: an unknown calendar is NotFound",
        is_not_found,
    );
    let r = dir.restore(early.id, SeriesScope::Whole).await;
    cx.ensure(
        r.is_ok(),
        "restore: the early one can be restored on its own once the calendar lives",
    );
}
