//! Writing with a series scope: whole, one occurrence, this and the following (dossier A9, A10).
//!
//! Pattern: Command executor. One function takes the checked change and commits it with a revision bump, a log
//! entry and a change-feed entry; the three scopes only differ in how the new state is made.

use chrono::{Duration, NaiveDateTime};

use crate::check;
use crate::draft::{
    Appointment, AppointmentPatch, Error, FieldChange, HistoryKind, LocalDateTime, ObjectId,
    Override, OverrideChange, Problem, Recurrence, Related, Relation, Result, Revision, RuleEnd,
    SeriesScope, Value,
};
use crate::instances::{expand, series_start, when_at};
use crate::patch::{apply, describe, merge};
use crate::recur::occurrences;
use crate::state::{Item, State};
use crate::zone::{local, naive};

pub(crate) fn invalid(field: &str, reason: &str) -> Error {
    Error::Invalid(vec![Problem {
        field: field.to_owned(),
        reason: reason.to_owned(),
    }])
}

/// The occurrence `rid` of a series, as wall-clock time.
///
/// # Errors
/// `Invalid` when the appointment is not a series or `rid` is not one of its occurrences.
#[allow(clippy::must_use_candidate)]
pub fn occurrence(a: &Appointment, rid: LocalDateTime) -> Result<NaiveDateTime> {
    if a.recurrence.is_empty() {
        return Err(invalid("scope", "this appointment is not a series"));
    }
    let (r, start) = (naive(rid), series_start(&a.when));
    let (Some(r), Some(start)) = (r, start) else {
        return Err(invalid("scope.recurrence_id", "no such date or time"));
    };
    if expand(&a.recurrence, start, r).contains(&r) {
        Ok(r)
    } else {
        Err(invalid(
            "scope.recurrence_id",
            "is not an occurrence of the series",
        ))
    }
}

pub(crate) fn checked(
    state: &State,
    id: ObjectId,
    expected: Option<Revision>,
) -> Result<Appointment> {
    let cur = state.live(id)?.appt.clone();
    state.writable_calendar(cur.calendar)?;
    match expected {
        Some(e) if e != cur.revision => Err(Error::Changed {
            current: cur.revision,
        }),
        _ => Ok(cur),
    }
}

pub(crate) fn commit(
    state: &mut State,
    old_rev: Revision,
    mut new: Appointment,
    kind: HistoryKind,
    changes: Vec<FieldChange>,
) -> Appointment {
    new.revision = Revision(old_rev.0 + 1);
    new.sequence += 1;
    new.updated = state.now;
    state.record(&new, kind, changes);
    state.items.insert(
        new.id,
        Item {
            appt: new.clone(),
            deleted: None,
            via_calendar: false,
        },
    );
    new
}

/// Cut the series so the occurrence `at` and all after it are gone; later overrides go too.
pub fn truncate(a: &mut Appointment, at: NaiveDateTime) {
    let until = RuleEnd::Until(local(at - Duration::seconds(1)));
    for r in &mut a.recurrence {
        if let Recurrence::Rule(rule) = r {
            rule.end = until;
        }
    }
    a.overrides
        .retain(|o| naive(o.recurrence_id).is_some_and(|n| n < at));
}

pub fn upsert(a: &mut Appointment, o: Override) {
    a.overrides.retain(|x| x.recurrence_id != o.recurrence_id);
    a.overrides.push(o);
    a.overrides.sort_by_key(|x| naive(x.recurrence_id));
}

/// A recurrence outside the subset (`Original`) is kept as it is; a new or changed one is `Unsupported` (ADR 0030).
///
/// # Errors
/// `Unsupported { capability }` when the patch brings an `Original` the appointment does not have.
pub fn no_new_original(
    cur: &Appointment,
    patch: &AppointmentPatch,
    capability: &str,
) -> Result<()> {
    let known = |r: &Recurrence| cur.recurrence.contains(r) || cur.excluded_recurrence.contains(r);
    let new = patch
        .recurrence
        .iter()
        .chain(&patch.excluded_recurrence)
        .flatten()
        .any(|r| matches!(r, Recurrence::Original(_)) && !known(r));
    if new {
        return Err(Error::Unsupported {
            capability: capability.to_owned(),
            reason: "a recurrence rule outside the subset cannot be made or changed".to_owned(),
        });
    }
    Ok(())
}

/// Change, invite or answer: the one write path. `expected = None` writes against the current revision.
///
/// # Errors
/// `NotFound`, `Changed`, `Invalid`, `UnknownZone`; nothing is written then.
pub fn write(
    state: &mut State,
    id: ObjectId,
    expected: Option<Revision>,
    patch: &AppointmentPatch,
    scope: SeriesScope,
    kind: HistoryKind,
    extra: Vec<FieldChange>,
) -> Result<Appointment> {
    let cur = checked(state, id, expected)?;
    let mut changes = describe(&cur, patch);
    changes.extend(extra);
    match scope {
        SeriesScope::Whole => {
            no_new_original(&cur, patch, "event.update")?;
            let mut new = cur.clone();
            apply(&mut new, patch);
            check::appointment(&new, &state.customs)?;
            if new == cur && changes.is_empty() {
                return Ok(cur);
            }
            Ok(commit(state, cur.revision, new, kind, changes))
        }
        SeriesScope::This { recurrence_id } => {
            if patch.recurrence.is_some() || patch.excluded_recurrence.is_some() {
                return Err(invalid("recurrence", "not allowed in an exception"));
            }
            occurrence(&cur, recurrence_id)?;
            let existing = cur
                .overrides
                .iter()
                .find(|o| o.recurrence_id == recurrence_id);
            let before = match existing.map(|o| &o.change) {
                Some(OverrideChange::Changed(p)) => p.clone(),
                _ => AppointmentPatch::default(),
            };
            let merged = merge(&before, patch);
            if (merged == before && existing.is_some())
                || (merged == AppointmentPatch::default() && changes.is_empty())
            {
                return Ok(cur);
            }
            let mut probe = cur.clone();
            apply(&mut probe, &merged);
            check::appointment(&probe, &state.customs)?;
            let mut new = cur.clone();
            upsert(
                &mut new,
                Override {
                    recurrence_id,
                    change: OverrideChange::Changed(merged),
                },
            );
            Ok(commit(state, cur.revision, new, kind, changes))
        }
        SeriesScope::ThisAndFollowing { recurrence_id } => {
            let at = occurrence(&cur, recurrence_id)?;
            if series_start(&cur.when) == Some(at) {
                return write(
                    state,
                    id,
                    expected,
                    patch,
                    SeriesScope::Whole,
                    kind,
                    changes,
                );
            }
            split(state, &cur, at, patch, kind, changes)
        }
    }
}

fn split(
    state: &mut State,
    cur: &Appointment,
    at: NaiveDateTime,
    patch: &AppointmentPatch,
    kind: HistoryKind,
    changes: Vec<FieldChange>,
) -> Result<Appointment> {
    let start = series_start(&cur.when).unwrap_or(at);
    let new_id = state.fresh_id();
    let number = crate::ids::object_number(new_id);
    let first_uid = cur
        .related
        .iter()
        .find(|r| r.relation == Relation::First)
        .map_or_else(|| cur.uid.clone(), |r| r.uid.clone());
    let mut tail = cur.clone();
    tail.id = new_id;
    tail.uid = format!("urn:uuid:00000000-0000-7000-9000-{number:012x}");
    tail.revision = Revision(1);
    tail.sequence = 0;
    tail.created = state.now;
    tail.updated = state.now;
    tail.when = when_at(&cur.when, at);
    tail.overrides
        .retain(|o| naive(o.recurrence_id).is_some_and(|n| n >= at));
    for (t, r) in tail.recurrence.iter_mut().zip(&cur.recurrence) {
        if let (Recurrence::Rule(t), Recurrence::Rule(r)) = (t, r) {
            if let RuleEnd::Count(c) = t.end {
                let before = occurrences(r, start, at - Duration::seconds(1)).len();
                t.end = RuleEnd::Count(c.saturating_sub(u32::try_from(before).unwrap_or(u32::MAX)));
            }
        }
    }
    tail.related.retain(|r| r.relation != Relation::First);
    tail.related.push(Related {
        uid: first_uid,
        relation: Relation::First,
    });
    apply(&mut tail, patch);
    check::appointment(&tail, &state.customs)?;
    let mut head = cur.clone();
    truncate(&mut head, at);
    head.related.retain(|r| r.relation != Relation::Next);
    head.related.push(Related {
        uid: tail.uid.clone(),
        relation: Relation::Next,
    });
    let cut = FieldChange {
        field: "recurrence.until".to_owned(),
        row_key: None,
        old: None,
        new: Some(Value::Local(local(at - Duration::seconds(1)))),
    };
    commit(state, cur.revision, head, kind, vec![cut]);
    state.record(&tail, HistoryKind::Create, changes);
    state.items.insert(
        tail.id,
        Item {
            appt: tail.clone(),
            deleted: None,
            via_calendar: false,
        },
    );
    Ok(tail)
}
