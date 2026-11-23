//! Applying, merging and describing an [`AppointmentPatch`] (typed `PatchObject`).
//!
//! Pattern: none (pure functions over the draft types).

use crate::draft::{Appointment, AppointmentPatch, FieldChange, Value, When};

/// A set: duplicates fall together, the first of each stays (keywords and categories are sets, plan section 11).
fn set(v: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    v.into_iter().filter(|x| seen.insert(x.clone())).collect()
}

/// Set the fields the patch names; listed fields are replaced as a whole.
pub fn apply(appt: &mut Appointment, p: &AppointmentPatch) {
    let p = p.clone();
    if let Some(v) = p.title {
        appt.title = v;
    }
    if let Some((text, kind)) = p.description {
        appt.description = text;
        appt.description_type = kind;
    }
    if let Some(v) = p.locale {
        appt.locale = v;
    }
    if let Some(v) = p.when {
        appt.when = v;
    }
    if let Some(v) = p.status {
        appt.status = v;
    }
    if let Some(v) = p.free_busy {
        appt.free_busy = v;
    }
    if let Some(v) = p.privacy {
        appt.privacy = v;
    }
    if let Some(v) = p.priority {
        appt.priority = v;
    }
    if let Some(v) = p.color {
        appt.color = v;
    }
    if let Some(v) = p.keywords {
        appt.keywords = set(v);
    }
    if let Some(v) = p.categories {
        appt.categories = set(v);
    }
    if let Some(v) = p.locations {
        appt.locations = v;
    }
    if let Some(v) = p.virtual_locations {
        appt.virtual_locations = v;
    }
    if let Some(v) = p.links {
        appt.links = v;
    }
    if let Some(v) = p.participants {
        appt.participants = v;
    }
    if let Some(v) = p.alerts {
        appt.alerts = v;
    }
    if let Some(v) = p.use_default_alerts {
        appt.use_default_alerts = v;
    }
    if let Some(v) = p.recurrence {
        appt.recurrence = v;
    }
    if let Some(v) = p.excluded_recurrence {
        appt.excluded_recurrence = v;
    }
    if let Some(v) = p.localizations {
        appt.localizations = v;
    }
}

/// `newer` on top of `older`: a field named by `newer` wins.
#[must_use]
pub fn merge(older: &AppointmentPatch, newer: &AppointmentPatch) -> AppointmentPatch {
    let (o, n) = (older.clone(), newer.clone());
    AppointmentPatch {
        title: n.title.or(o.title),
        description: n.description.or(o.description),
        locale: n.locale.or(o.locale),
        when: n.when.or(o.when),
        status: n.status.or(o.status),
        free_busy: n.free_busy.or(o.free_busy),
        privacy: n.privacy.or(o.privacy),
        priority: n.priority.or(o.priority),
        color: n.color.or(o.color),
        keywords: n.keywords.or(o.keywords),
        categories: n.categories.or(o.categories),
        locations: n.locations.or(o.locations),
        virtual_locations: n.virtual_locations.or(o.virtual_locations),
        links: n.links.or(o.links),
        participants: n.participants.or(o.participants),
        alerts: n.alerts.or(o.alerts),
        use_default_alerts: n.use_default_alerts.or(o.use_default_alerts),
        recurrence: n.recurrence.or(o.recurrence),
        excluded_recurrence: n.excluded_recurrence.or(o.excluded_recurrence),
        localizations: n.localizations.or(o.localizations),
    }
}

fn text(field: &str, old: &str, new: &str) -> Option<FieldChange> {
    (old != new).then(|| FieldChange {
        field: field.to_owned(),
        row_key: None,
        old: Some(Value::Text(old.to_owned())),
        new: Some(Value::Text(new.to_owned())),
    })
}

/// The typed field changes the patch makes to `before` (only scalar fields and the time; lists are one change
/// per list with the count as the value: the fake does not diff rows).
#[must_use]
pub fn describe(before: &Appointment, p: &AppointmentPatch) -> Vec<FieldChange> {
    let mut after = before.clone();
    apply(&mut after, p);
    let mut out: Vec<FieldChange> = Vec::new();
    out.extend(text("title", &before.title, &after.title));
    out.extend(text("description", &before.description, &after.description));
    out.extend(text(
        "status",
        &format!("{:?}", before.status),
        &format!("{:?}", after.status),
    ));
    out.extend(text(
        "privacy",
        &format!("{:?}", before.privacy),
        &format!("{:?}", after.privacy),
    ));
    out.extend(when_changes(&before.when, &after.when));
    for (field, a, b) in [
        (
            "participants",
            before.participants.len(),
            after.participants.len(),
        ),
        ("alerts", before.alerts.len(), after.alerts.len()),
        ("locations", before.locations.len(), after.locations.len()),
        (
            "recurrence",
            before.recurrence.len(),
            after.recurrence.len(),
        ),
    ] {
        if a != b {
            out.push(FieldChange {
                field: field.to_owned(),
                row_key: None,
                old: Some(Value::Int(i64::try_from(a).unwrap_or(0))),
                new: Some(Value::Int(i64::try_from(b).unwrap_or(0))),
            });
        }
    }
    out
}

fn when_changes(a: &When, b: &When) -> Vec<FieldChange> {
    let mut out = Vec::new();
    let change = |field: &str, old: Option<Value>, new: Option<Value>| FieldChange {
        field: field.to_owned(),
        row_key: None,
        old,
        new,
    };
    match (a, b) {
        (
            When::Timed {
                start: s1,
                zone: z1,
                ..
            },
            When::Timed {
                start: s2,
                zone: z2,
                ..
            },
        ) => {
            if s1 != s2 {
                out.push(change(
                    "when.start_local",
                    Some(Value::Local(*s1)),
                    Some(Value::Local(*s2)),
                ));
            }
            if z1 != z2 {
                out.push(change(
                    "when.zone",
                    Some(Value::Zone(z1.clone())),
                    Some(Value::Zone(z2.clone())),
                ));
            }
        }
        (When::Floating { start: s1, .. }, When::Floating { start: s2, .. }) => {
            if s1 != s2 {
                out.push(change(
                    "when.start_local",
                    Some(Value::Local(*s1)),
                    Some(Value::Local(*s2)),
                ));
            }
        }
        (When::AllDay { first: f1, .. }, When::AllDay { first: f2, .. }) => {
            if f1 != f2 {
                out.push(change(
                    "when.start_date",
                    Some(Value::Date(*f1)),
                    Some(Value::Date(*f2)),
                ));
            }
        }
        _ => out.push(change("when.kind", Some(kind(a)), Some(kind(b)))),
    }
    if a != b && out.is_empty() {
        out.push(change("when.length", None, None));
    }
    out
}

fn kind(w: &When) -> Value {
    Value::Text(
        match w {
            When::Timed { .. } => "timed",
            When::Floating { .. } => "floating",
            When::AllDay { .. } => "all-day",
        }
        .to_owned(),
    )
}

/// Wall-clock start of a `When` as a `Value` (for a proposal).
#[must_use]
pub fn start_value(w: &When) -> Value {
    match w {
        When::Timed { start, .. } | When::Floating { start, .. } => Value::Local(*start),
        When::AllDay { first, .. } => Value::Date(*first),
    }
}
