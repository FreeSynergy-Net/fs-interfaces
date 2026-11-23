//! Rules for `purge`: the way out of "never delete automatically", and what a retention does to it.
//!
//! Pattern: Template method (rules as plain checks collected in the report).

use super::cx::{Cx, forbidden, invalid_at, is_not_found};
use crate::draft::{ObjectId, Restorable, SeriesScope, Trash, Utc};

const DAY: i64 = 86_400_000;

pub(super) async fn purge(cx: &mut Cx<'_>) {
    let dir = cx.dir;
    let Some(c) = cx.calendar("purge").await else {
        return;
    };
    let (Some(live), Some(gone)) = (
        cx.appointment(c.id, "live", 2).await,
        cx.appointment(c.id, "gone", 3).await,
    ) else {
        return;
    };
    let r = dir.purge(live.id).await;
    cx.fails(
        &r,
        "purge: a living appointment is Invalid",
        invalid_at("id", None),
    );
    let r = dir.purge(c.id).await;
    cx.fails(
        &r,
        "purge: a living calendar is Invalid",
        invalid_at("id", None),
    );
    let r = dir.purge(crate::ids::object_id(u64::MAX)).await;
    cx.fails(&r, "purge: an unknown id is NotFound", is_not_found);
    if dir
        .delete(gone.id, gone.revision, SeriesScope::Whole)
        .await
        .is_err()
    {
        cx.ensure(false, "delete: answers");
        return;
    }
    cx.ensure(
        dir.purge(gone.id).await.is_ok(),
        "purge: a trashed appointment is deleted for good",
    );
    cx.ensure(
        cx.all(&[c.id], Trash::Include)
            .await
            .iter()
            .all(|s| s.id != gone.id),
        "purge: it is in no trash view any more",
    );
    let r = dir.history(gone.id, None, 10).await;
    cx.fails(
        &r,
        "purge: its log is gone with it (history is NotFound)",
        is_not_found,
    );
    let r = dir.restore(gone.id, SeriesScope::Whole).await;
    cx.fails(&r, "purge: restore afterwards is NotFound", is_not_found);
    let r = dir.purge(gone.id).await;
    cx.fails(&r, "purge: a second purge is NotFound", is_not_found);
    // A whole calendar with its appointments.
    let Some(k) = cx.calendar("purge-calendar").await else {
        return;
    };
    let Some(x) = cx.appointment(k.id, "x", 4).await else {
        return;
    };
    let _ = dir.delete_calendar(k.id, k.revision).await;
    cx.ensure(
        dir.purge(k.id).await.is_ok(),
        "purge: a trashed calendar is deleted for good",
    );
    cx.ensure(
        cx.listed(k.id).await.is_none(),
        "purge: the calendar is in no view any more",
    );
    let r = dir.history(x.id, None, 10).await;
    cx.fails(
        &r,
        "purge: the log of its appointments goes too",
        is_not_found,
    );
    let r = dir.restore_calendar(k.id).await;
    cx.fails(&r, "restore_calendar after purge is NotFound", is_not_found);
}

/// The `trash` of an appointment as the trash view shows it.
async fn trash_of(cx: &Cx<'_>, cal: ObjectId, id: ObjectId) -> Option<Restorable> {
    let all = cx.all(&[cal], Trash::Only).await;
    all.into_iter().find(|s| s.id == id).and_then(|s| s.trash)
}

/// A retention beats everything: `Held`, `purge` is `Forbidden` and deletes nothing.
pub(super) async fn retention(cx: &mut Cx<'_>) {
    let dir = cx.dir;
    let (Some(now), true) = (cx.fx.now(), cx.fx.set_trash_period(Some(30 * DAY)).await) else {
        cx.skip("retention: the fixture cannot set the period or tell the time");
        return;
    };
    let Some(c) = cx.calendar("held").await else {
        return;
    };
    let (Some(a), Some(b), Some(d)) = (
        cx.appointment(c.id, "a", 2).await,
        cx.appointment(c.id, "b", 3).await,
        cx.appointment(c.id, "d", 4).await,
    ) else {
        return;
    };
    let mut recs = Vec::new();
    for t in [&a, &b, &d] {
        let Ok(x) = dir.delete(t.id, t.revision, SeriesScope::Whole).await else {
            cx.ensure(false, "delete: answers");
            return;
        };
        recs.push(x.restorable);
    }
    let end = Utc::from_millis(now.millis() + 90 * DAY);
    let soon = Utc::from_millis(now.millis() + 10 * DAY);
    if !(cx.fx.hold(a.id, None).await
        && cx.fx.hold(b.id, Some(end)).await
        && cx.fx.hold(d.id, Some(soon)).await)
    {
        cx.skip("retention: the fixture cannot keep an object");
        return;
    }
    cx.ensure(
        trash_of(cx, c.id, a.id).await == Some(Restorable::Held(None)),
        "restorable: a retention without end is Held(None)",
    );
    cx.ensure(
        trash_of(cx, c.id, b.id).await == Some(Restorable::Held(Some(end))),
        "restorable: Held(Some(t)) is the later of period and retention (retention later)",
    );
    let Restorable::Until(frame) = recs[2] else {
        cx.ensure(false, "delete: with a period it is Until");
        return;
    };
    cx.ensure(
        trash_of(cx, c.id, d.id).await == Some(Restorable::Held(Some(frame))),
        "restorable: Held(Some(t)) is the later of period and retention (period later)",
    );
    for t in [&a, &b, &d] {
        let r = dir.purge(t.id).await;
        cx.fails(
            &r,
            "purge: a kept appointment is Forbidden(trash.purge)",
            forbidden("trash.purge"),
        );
    }
    cx.ensure(
        trash_of(cx, c.id, a.id).await.is_some(),
        "purge: a refused purge deletes nothing",
    );
    // The calendar: one kept appointment inside keeps the whole calendar.
    let Some(k) = cx.calendar("held-calendar").await else {
        return;
    };
    let Some(y) = cx.appointment(k.id, "y", 5).await else {
        return;
    };
    let _ = dir.delete_calendar(k.id, k.revision).await;
    if cx.fx.hold(y.id, None).await {
        let r = dir.purge(k.id).await;
        cx.fails(
            &r,
            "purge: a calendar with a kept appointment is Forbidden(trash.purge)",
            forbidden("trash.purge"),
        );
        cx.ensure(
            cx.listed(k.id).await.is_some(),
            "purge: the refused purge leaves the calendar",
        );
        let r = dir.purge(y.id).await;
        cx.fails(
            &r,
            "purge: the kept appointment of a trashed calendar is Forbidden too",
            forbidden("trash.purge"),
        );
    }
    // A retention that has ended does not count.
    if cx
        .fx
        .hold(a.id, Some(Utc::from_millis(now.millis() - DAY)))
        .await
    {
        cx.ensure(
            dir.purge(a.id).await.is_ok(),
            "purge: a retention that has ended does not hold",
        );
    }
}

/// "Never delete automatically" (`Manual`), the retention with an end on top of it (K8), and the deadline.
pub(super) async fn period(cx: &mut Cx<'_>) {
    let dir = cx.dir;
    let Some(now) = cx.fx.now() else {
        cx.skip("period: the fixture cannot tell the time");
        return;
    };
    if !cx.fx.set_trash_period(None).await {
        cx.skip("period: the fixture cannot set 'never automatically'");
        return;
    }
    let Some(c) = cx.calendar("never").await else {
        return;
    };
    let (Some(a), Some(b)) = (
        cx.appointment(c.id, "a", 2).await,
        cx.appointment(c.id, "b", 3).await,
    ) else {
        return;
    };
    let da = dir.delete(a.id, a.revision, SeriesScope::Whole).await;
    cx.ensure(
        da.as_ref()
            .is_ok_and(|d| d.restorable == Restorable::Manual),
        "delete: 'never automatically' is Manual",
    );
    let Ok(db) = dir.delete(b.id, b.revision, SeriesScope::Whole).await else {
        return;
    };
    let end = Utc::from_millis(now.millis() + 10 * DAY);
    if cx.fx.hold(b.id, Some(end)).await {
        let only = cx.all(&[c.id], Trash::Only).await;
        cx.ensure(
            only.iter().any(|s| s.id == b.id && s.trash == Some(Restorable::Held(Some(end)))),
            "restorable: 'never automatically' plus a retention WITH an end is Held(Some(end)), not Never/Manual",
        );
    }
    let _ = db;
    if cx.advance(3650 * DAY, "Manual").await {
        cx.ensure(
            cx.all(&[c.id], Trash::Only)
                .await
                .iter()
                .any(|s| s.id == a.id && s.trash == Some(Restorable::Manual)),
            "Manual: nothing goes away by itself, however long it lies there",
        );
        cx.ensure(
            cx.all(&[c.id], Trash::Only)
                .await
                .iter()
                .any(|s| s.id == b.id && s.trash == Some(Restorable::Manual)),
            "restorable: after the end of the retention it is Manual again",
        );
        cx.ensure(
            dir.purge(a.id).await.is_ok(),
            "purge: the way out of Manual",
        );
    }
    // The deadline: after the period everything is gone for good.
    if !cx.fx.set_trash_period(Some(30 * DAY)).await {
        return;
    }
    let Some(c2) = cx.calendar("deadline").await else {
        return;
    };
    let Some(t) = cx.appointment(c2.id, "soon gone", 4).await else {
        return;
    };
    let Ok(d) = dir.delete(t.id, t.revision, SeriesScope::Whole).await else {
        return;
    };
    cx.ensure(
        matches!(d.restorable, Restorable::Until(_)),
        "delete: a period gives Until",
    );
    if cx.advance(31 * DAY, "restore: after the deadline").await {
        let r = dir.restore(t.id, SeriesScope::Whole).await;
        cx.fails(
            &r,
            "restore: after the deadline the appointment is gone (NotFound)",
            is_not_found,
        );
        cx.ensure(
            cx.all(&[c2.id], Trash::Only).await.is_empty(),
            "list(Only): nothing after the deadline",
        );
    }
}
