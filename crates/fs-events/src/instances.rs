//! Turning one appointment into the occurrences that overlap a window.
//!
//! Pattern: Template method (one skeleton, the series and the single case differ in the candidate list).
//! A damaged appointment costs only itself: an error becomes a [`Damaged`] entry.

use std::collections::{HashMap, HashSet};

use chrono::{Duration, NaiveDateTime};

use crate::draft::{
    Appointment, AppointmentPatch, Damaged, Instance, LocalDateTime, Override, OverrideChange,
    PersonRef, Recurrence, Result, Utc, When, Zone,
};
use crate::ids::actor;
use crate::patch::apply;
use crate::recur::occurrences;
use crate::zone::{CustomZones, bounds, from_utc_ms, local, naive, naive_date};

/// The wall-clock start of the first occurrence (an all-day one counts from midnight).
#[must_use]
pub fn series_start(when: &When) -> Option<NaiveDateTime> {
    match when {
        When::Timed { start, .. } | When::Floating { start, .. } => naive(*start),
        When::AllDay { first, .. } => {
            naive_date(*first).map(|d| d.and_time(chrono::NaiveTime::MIN))
        }
    }
}

/// The `When` of the occurrence that starts at `at` (same zone, length, flags).
#[must_use]
pub fn when_at(base: &When, at: NaiveDateTime) -> When {
    match base {
        When::Timed {
            zone,
            length,
            end_zone,
            zone_assumed,
            ..
        } => When::Timed {
            start: local(at),
            zone: zone.clone(),
            length: *length,
            end_zone: end_zone.clone(),
            zone_assumed: *zone_assumed,
        },
        When::Floating { length, .. } => When::Floating {
            start: local(at),
            length: *length,
        },
        When::AllDay { days, .. } => When::AllDay {
            first: local(at).date(),
            days: *days,
        },
    }
}

fn instance(
    a: &Appointment,
    rid: Option<LocalDateTime>,
    when: When,
    patch: Option<&AppointmentPatch>,
    viewer: Option<&PersonRef>,
    customs: &CustomZones,
    view: &Zone,
) -> Result<Instance> {
    let mut eff = a.clone();
    if let Some(p) = patch {
        apply(&mut eff, p);
    }
    eff.when = when;
    let (start, end) = bounds(&eff.when, customs, view)?;
    let my_status = viewer
        .and_then(|v| {
            eff.participants
                .iter()
                .find(|p| actor(p).is_some_and(|x| x.id == v.id))
        })
        .map(|p| p.status);
    Ok(Instance {
        appointment: a.id,
        calendar: a.calendar,
        recurrence_id: rid,
        title: eff.title,
        when: eff.when,
        start,
        end,
        status: eff.status,
        free_busy: eff.free_busy,
        privacy: eff.privacy,
        overridden: patch.is_some(),
        recurring: !a.recurrence.is_empty(),
        participant_count: u32::try_from(eff.participants.len()).unwrap_or(u32::MAX),
        my_status,
    })
}

fn overlaps(i: &Instance, from: Utc, to: Utc) -> bool {
    if i.end > i.start {
        i.start < to && i.end > from
    } else {
        i.start >= from && i.start < to
    }
}

/// The occurrences of `a` that overlap `[from, to)`, and what could not be answered.
#[must_use]
pub fn in_window(
    a: &Appointment,
    customs: &CustomZones,
    viewer: Option<&PersonRef>,
    view: &Zone,
    from: Utc,
    to: Utc,
) -> (Vec<Instance>, Vec<Damaged>) {
    let mut found = Vec::new();
    let mut damaged = Vec::new();
    let mut fail = |rid: Option<LocalDateTime>, e: &crate::draft::Error| {
        damaged.push(Damaged {
            appointment: a.id,
            recurrence_id: rid,
            reason: format!("{e:?}"),
            cut: false,
        });
    };
    if a.recurrence.is_empty() && a.overrides.is_empty() {
        match instance(a, None, a.when.clone(), None, viewer, customs, view) {
            Ok(i) if overlaps(&i, from, to) => found.push(i),
            Ok(_) => {}
            Err(e) => fail(None, &e),
        }
        return (found, damaged);
    }
    let Some(start) = series_start(&a.when) else {
        fail(None, &crate::draft::Error::Invalid(Vec::new()));
        return (found, damaged);
    };
    let limit = from_utc_ms(to) + Duration::days(2);
    let all = expand(&a.recurrence, start, limit);
    let excluded: HashSet<NaiveDateTime> = a
        .excluded_recurrence
        .iter()
        .filter_map(Recurrence::rule)
        .flat_map(|r| occurrences(r, start, limit))
        .collect();
    let overrides: HashMap<NaiveDateTime, &Override> = a
        .overrides
        .iter()
        .filter_map(|o| naive(o.recurrence_id).map(|n| (n, o)))
        .collect();
    for (at, o) in &overrides {
        if let OverrideChange::Changed(p) = &o.change {
            let base = when_at(&a.when, *at);
            match instance(
                a,
                Some(o.recurrence_id),
                p.when.clone().unwrap_or(base),
                Some(p),
                viewer,
                customs,
                view,
            ) {
                Ok(i) if overlaps(&i, from, to) => found.push(i),
                Ok(_) => {}
                Err(e) => fail(Some(o.recurrence_id), &e),
            }
        }
    }
    for at in all
        .into_iter()
        .filter(|at| !excluded.contains(at) && !overrides.contains_key(at))
    {
        match instance(
            a,
            Some(local(at)),
            when_at(&a.when, at),
            None,
            viewer,
            customs,
            view,
        ) {
            Ok(i) if overlaps(&i, from, to) => found.push(i),
            Ok(_) => {}
            Err(e) => fail(Some(local(at)), &e),
        }
    }
    (found, damaged)
}

/// All occurrences of a list of recurrences from `start` (always the first) up to `limit`, sorted and without
/// doubles. An `Original` rule is not expanded (it shows the first occurrence only, ADR 0030).
#[must_use]
pub fn expand(
    rules: &[Recurrence],
    start: NaiveDateTime,
    limit: NaiveDateTime,
) -> Vec<NaiveDateTime> {
    let mut all = vec![start];
    for rule in rules.iter().filter_map(Recurrence::rule) {
        all.extend(occurrences(rule, start, limit));
    }
    all.sort();
    all.dedup();
    all
}
