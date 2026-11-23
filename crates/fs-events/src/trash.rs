//! Delete and restore: the trash for a whole appointment, an `Excluded` override or a cut for a series scope.
//!
//! Pattern: Command executor (shares the commit path of `scope.rs`).

use crate::draft::{
    Appointment, Deleted, Error, HistoryKind, ObjectId, Override, OverrideChange, Result,
    SeriesScope, Utc,
};
use crate::instances::series_start;
use crate::scope::{checked, commit, invalid, occurrence, truncate, upsert};
use crate::state::Cut;
use crate::state::{Item, State};

/// Delete with a scope: the whole thing goes to the trash; one occurrence becomes an `Excluded` override;
/// this and the following cut the series.
///
/// # Errors
/// `NotFound`, `Changed`, `Invalid`.
pub fn delete(
    state: &mut State,
    id: ObjectId,
    expected: crate::draft::Revision,
    scope: SeriesScope,
) -> Result<Deleted> {
    let cur = checked(state, id, Some(expected))?;
    let whole = |state: &mut State, cur: &Appointment| {
        let at = state.now;
        state.items.insert(
            cur.id,
            Item {
                appt: cur.clone(),
                deleted: Some(at),
                via_calendar: false,
            },
        );
        state.record(cur, HistoryKind::Delete, Vec::new());
        Deleted {
            id: cur.id,
            scope,
            restorable: state.restorable(&[cur.id], at),
        }
    };
    match scope {
        SeriesScope::Whole => Ok(whole(state, &cur)),
        SeriesScope::This { recurrence_id } => {
            occurrence(&cur, recurrence_id)?;
            let until = cut_end(state);
            let prior: Vec<Override> = cur
                .overrides
                .iter()
                .filter(|o| o.recurrence_id == recurrence_id)
                .cloned()
                .collect();
            let mut new = cur.clone();
            upsert(
                &mut new,
                Override {
                    recurrence_id,
                    change: OverrideChange::Excluded,
                },
            );
            let done = commit(state, cur.revision, new, HistoryKind::Delete, Vec::new());
            state.cuts.push(Cut {
                id,
                recurrence_id,
                following: false,
                until,
                rev_after: done.revision,
                recurrence: Vec::new(),
                overrides: prior,
            });
            Ok(Deleted {
                id,
                scope,
                restorable: state.restorable(&[], state.now),
            })
        }
        SeriesScope::ThisAndFollowing { recurrence_id } => {
            let at = occurrence(&cur, recurrence_id)?;
            if series_start(&cur.when) == Some(at) {
                return Ok(whole(state, &cur));
            }
            let until = cut_end(state);
            let mut new = cur.clone();
            truncate(&mut new, at);
            let removed = cur
                .overrides
                .iter()
                .filter(|o| !new.overrides.contains(o))
                .cloned()
                .collect();
            let done = commit(state, cur.revision, new, HistoryKind::Delete, Vec::new());
            state.cuts.push(Cut {
                id,
                recurrence_id,
                following: true,
                until,
                rev_after: done.revision,
                recurrence: cur.recurrence.clone(),
                overrides: removed,
            });
            Ok(Deleted {
                id,
                scope,
                restorable: state.restorable(&[], state.now),
            })
        }
    }
}

/// When a partial delete (`This`, `ThisAndFollowing`) can no longer be undone: the period (far away for "never").
fn cut_end(state: &State) -> Utc {
    Utc::from_millis(
        state
            .now
            .millis()
            .saturating_add(state.period_ms.unwrap_or(i64::MAX / 4)),
    )
}

/// Bring a deletion back while the trash still holds it. `Whole` (or any scope for a trashed appointment,
/// which also covers `ThisAndFollowing` on the first occurrence) takes it out of the trash; `This` removes
/// the `Excluded` override; `ThisAndFollowing` gives back the rules and overrides the cut took away.
///
/// # Errors
/// `NotFound` for an unknown id or one whose period is over; `Invalid` when there is nothing to undo or the
/// calendar is in the trash; `ReadOnly` in a subscription.
pub fn restore(state: &mut State, id: ObjectId, scope: SeriesScope) -> Result<Appointment> {
    let item = state.items.get(&id).ok_or(Error::NotFound(id))?.clone();
    if !state.may_see(item.appt.calendar) {
        return Err(Error::NotFound(id));
    }
    if state.cal_deleted.contains_key(&item.appt.calendar) {
        return Err(invalid(
            "calendar",
            "is in the trash: restore the calendar first",
        ));
    }
    if let crate::draft::CalendarSource::Subscribed(_) = state.calendar(item.appt.calendar)?.source
    {
        return Err(Error::ReadOnly {
            calendar: item.appt.calendar,
        });
    }
    if item.deleted.is_some() {
        return Ok(commit(
            state,
            item.appt.revision,
            item.appt,
            HistoryKind::Undo,
            Vec::new(),
        ));
    }
    let rid = match scope {
        SeriesScope::Whole => return Err(invalid("id", "is not in the trash")),
        SeriesScope::This { recurrence_id } | SeriesScope::ThisAndFollowing { recurrence_id } => {
            recurrence_id
        }
    };
    let following = matches!(scope, SeriesScope::ThisAndFollowing { .. });
    let Some(pos) = state
        .cuts
        .iter()
        .position(|c| c.id == id && c.recurrence_id == rid && c.following == following)
    else {
        return Err(invalid("scope", "nothing was deleted there"));
    };
    if state.now.millis() >= state.cuts[pos].until.millis() {
        state.cuts.remove(pos);
        return Err(Error::NotFound(id));
    }
    if following && state.cuts[pos].rev_after != item.appt.revision {
        return Err(Error::Changed {
            current: item.appt.revision,
        });
    }
    let cut = state.cuts.remove(pos);
    let mut new = item.appt.clone();
    if following {
        new.recurrence = cut.recurrence;
        for o in cut.overrides {
            upsert(&mut new, o);
        }
    } else {
        new.overrides
            .retain(|o| !(o.recurrence_id == rid && o.change == OverrideChange::Excluded));
        for o in cut.overrides {
            upsert(&mut new, o);
        }
    }
    Ok(commit(
        state,
        item.appt.revision,
        new,
        HistoryKind::Undo,
        Vec::new(),
    ))
}
