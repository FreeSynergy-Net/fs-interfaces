//! Calendars: create, change, delete (with everything in it) and bring back.
//!
//! Pattern: Facade over the state (owner and AI rules are checked here; the trash arithmetic is in `trashing.rs`).

use crate::draft::{
    CalendarDeleted, CalendarInfo, CalendarPatch, CalendarSource, Change, Error, FieldChange,
    HistoryKind, ObjectId, PersonArt, PersonRef, Problem, Refresh, Result, Revision, Subscription,
    Value, Zone,
};
use crate::fake::FakeAppointments;
use crate::scope::invalid;
use crate::state::State;
use crate::zone::resolve_at;

/// The shortest repeat time of a subscription this fake accepts (seconds).
pub const MIN_REFRESH: u32 = 300;

fn forbidden(capability: &str) -> Error {
    Error::Forbidden {
        capability: capability.to_owned(),
    }
}

/// Who may own a calendar: not a `Profile` (`Invalid`), never an AI account (`Forbidden`, like `tasks_v1`).
fn owner_ok(s: &State, owner: &PersonRef, capability: &str) -> Result<()> {
    if owner.art == PersonArt::Profile {
        return Err(Error::Invalid(vec![Problem {
            field: "owner".to_owned(),
            reason: "owner.profile".to_owned(),
        }]));
    }
    if s.ai_people.contains(&owner.id.canonical()) {
        return Err(forbidden(capability));
    }
    Ok(())
}

pub(crate) fn refresh_ok(refresh: Refresh) -> Result<()> {
    match refresh {
        Refresh::Every { seconds } if seconds < MIN_REFRESH => Err(Error::Invalid(vec![Problem {
            field: "refresh".to_owned(),
            reason: "refresh.too-short".to_owned(),
        }])),
        _ => Ok(()),
    }
}

use crate::draft::BasicValue;

impl FakeAppointments {
    pub(crate) fn create_calendar_impl(
        &self,
        name: &str,
        color: Option<&str>,
        default_zone: Option<Zone>,
        owner: Option<PersonRef>,
    ) -> Result<CalendarInfo> {
        let mut s = self.lock();
        if s.asker_is_ai() {
            return Err(forbidden("calendar.create"));
        }
        if let Some(o) = &owner {
            owner_ok(&s, o, "calendar.create")?;
        }
        if name.trim().is_empty() {
            return Err(invalid("name", "must not be empty"));
        }
        if let Some(z) = &default_zone {
            resolve_at(z, &s.customs, "default_zone")?;
        }
        let info = CalendarInfo {
            id: s.fresh_id(),
            name: name.to_owned(),
            color: color.map(str::to_owned),
            default_zone,
            owner: owner.unwrap_or_else(|| s.viewer.clone()),
            source: CalendarSource::Own,
            trash: None,
            revision: Revision(1),
        };
        s.calendars.push(info.clone());
        s.record_calendar(info.id, HistoryKind::Create, Vec::new());
        Ok(s.calendar_view(&info))
    }

    pub(crate) fn update_calendar_impl(
        &self,
        id: ObjectId,
        expected: Revision,
        change: &CalendarPatch,
    ) -> Result<CalendarInfo> {
        let mut s = self.lock();
        let cur = s.calendar(id)?.clone();
        if s.cal_deleted.contains_key(&id) {
            return Err(Error::NotFound(id));
        }
        if s.asker_is_ai() {
            return Err(forbidden("calendar.update"));
        }
        if cur.revision != expected {
            return Err(Error::Changed {
                current: cur.revision,
            });
        }
        if let Some(o) = &change.owner {
            owner_ok(&s, o, "calendar.update")?;
        }
        let mut new = cur.clone();
        let mut lines = Vec::new();
        if let Some(n) = &change.name {
            if n.trim().is_empty() {
                return Err(invalid("name", "must not be empty"));
            }
            lines.push(text_line("name", &cur.name, n));
            new.name.clone_from(n);
        }
        if let Some(c) = &change.color {
            new.color.clone_from(c);
        }
        if let Some(z) = &change.default_zone {
            if let Some(z) = z {
                resolve_at(z, &s.customs, "default_zone")?;
            }
            new.default_zone.clone_from(z);
        }
        if let Some(o) = &change.owner {
            lines.push(FieldChange {
                field: "owner".to_owned(),
                row_key: None,
                old: Some(Value::Person(cur.owner.clone())),
                new: Some(Value::Person(o.clone())),
            });
            new.owner = o.clone();
        }
        if change.url.is_some() || change.refresh.is_some() {
            let CalendarSource::Subscribed(sub) = &mut new.source else {
                let field = if change.url.is_some() {
                    "url"
                } else {
                    "refresh"
                };
                return Err(invalid(field, "only a subscription has it"));
            };
            if let Some(u) = &change.url {
                if let Some(code) = crate::ssrf::refusal(u, &s.dns) {
                    return Err(invalid("url", code));
                }
                // The address is a secret: the log only says "changed", never the value.
                lines.push(FieldChange {
                    field: "url".to_owned(),
                    row_key: None,
                    old: None,
                    new: None,
                });
                *sub = Subscription {
                    url: Some(u.clone()),
                    ..sub.clone()
                };
            }
            if let Some(r) = change.refresh {
                refresh_ok(r)?;
                sub.refresh = r;
            }
        }
        new.revision = Revision(cur.revision.0 + 1);
        s.calendars
            .iter_mut()
            .filter(|c| c.id == id)
            .for_each(|c| *c = new.clone());
        s.record_calendar(id, HistoryKind::Update, lines);
        Ok(s.calendar_view(&new))
    }

    pub(crate) fn delete_calendar_impl(
        &self,
        id: ObjectId,
        expected: Revision,
    ) -> Result<CalendarDeleted> {
        let mut s = self.lock();
        let cur = s.calendar(id)?.clone();
        if s.asker_is_ai() {
            return Err(forbidden("calendar.delete"));
        }
        if s.cal_deleted.contains_key(&id) {
            return Err(Error::NotFound(id));
        }
        if cur.revision != expected {
            return Err(Error::Changed {
                current: cur.revision,
            });
        }
        let at = s.now;
        let living: Vec<ObjectId> = s
            .items
            .values()
            .filter(|i| i.appt.calendar == id && i.deleted.is_none())
            .map(|i| i.appt.id)
            .collect();
        for item_id in &living {
            if let Some(i) = s.items.get_mut(item_id) {
                i.deleted = Some(at);
                i.via_calendar = true;
            }
            s.changes.push(Change {
                appointment: Some(*item_id),
                calendar: id,
                kind: HistoryKind::Delete,
                at,
            });
        }
        s.cal_deleted.insert(id, at);
        s.record_calendar(id, HistoryKind::Delete, Vec::new());
        Ok(CalendarDeleted {
            id,
            restorable: s.restorable(&s.calendar_ids(id), at),
            appointments: u32::try_from(living.len()).unwrap_or(u32::MAX),
        })
    }

    pub(crate) fn restore_calendar_impl(&self, id: ObjectId) -> Result<CalendarInfo> {
        let mut s = self.lock();
        let cur = s.calendar(id)?.clone();
        let Some(at) = s.cal_deleted.get(&id).copied() else {
            return Err(invalid("id", "is not in the trash"));
        };
        s.cal_deleted.remove(&id);
        let back: Vec<ObjectId> = s
            .items
            .values()
            .filter(|i| i.appt.calendar == id && i.via_calendar && i.deleted == Some(at))
            .map(|i| i.appt.id)
            .collect();
        let now = s.now;
        for item_id in back {
            if let Some(i) = s.items.get_mut(&item_id) {
                i.deleted = None;
                i.via_calendar = false;
            }
            s.changes.push(Change {
                appointment: Some(item_id),
                calendar: id,
                kind: HistoryKind::Undo,
                at: now,
            });
        }
        s.record_calendar(id, HistoryKind::Undo, Vec::new());
        Ok(s.calendar_view(&cur))
    }
}

fn text_line(field: &str, old: &str, new: &str) -> FieldChange {
    FieldChange {
        field: field.to_owned(),
        row_key: None,
        old: Some(Value::Text(old.to_owned())),
        new: Some(Value::Text(new.to_owned())),
    }
}
