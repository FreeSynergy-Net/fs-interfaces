//! `purge`: delete a trashed appointment or calendar for good (the way out of "never delete automatically").
//!
//! Pattern: Command (one function; the retention beats everything, an AI never purges).

use crate::draft::{Change, Error, HistoryKind, ObjectId, Result};
use crate::fake::FakeAppointments;
use crate::scope::invalid;

fn forbidden() -> Error {
    Error::Forbidden {
        capability: "trash.purge".to_owned(),
    }
}

impl FakeAppointments {
    pub(crate) fn purge_impl(&self, id: ObjectId) -> Result<()> {
        let mut s = self.lock();
        if s.asker_is_ai() {
            return Err(forbidden());
        }
        let now = s.now;
        if let Some(item) = s.items.get(&id).cloned() {
            if !s.may_see(item.appt.calendar) {
                return Err(Error::NotFound(id));
            }
            if item.deleted.is_none() {
                return Err(invalid("id", "is not in the trash"));
            }
            let mut ids = vec![id];
            if item.via_calendar {
                ids.push(item.appt.calendar);
            }
            if s.active_hold(&ids).is_some() {
                return Err(forbidden());
            }
            s.remove_item(id);
            s.changes.push(Change {
                appointment: Some(id),
                calendar: item.appt.calendar,
                kind: HistoryKind::Delete,
                at: now,
            });
            return Ok(());
        }
        s.calendar(id)?;
        if !s.cal_deleted.contains_key(&id) {
            return Err(invalid("id", "is not in the trash"));
        }
        if s.active_hold(&s.calendar_ids(id)).is_some() {
            return Err(forbidden());
        }
        let gone: Vec<ObjectId> = s
            .items
            .values()
            .filter(|i| i.appt.calendar == id)
            .map(|i| i.appt.id)
            .collect();
        s.remove_calendar(id);
        for appointment in gone {
            s.changes.push(Change {
                appointment: Some(appointment),
                calendar: id,
                kind: HistoryKind::Delete,
                at: now,
            });
        }
        s.changes.push(Change {
            appointment: None,
            calendar: id,
            kind: HistoryKind::Delete,
            at: now,
        });
        Ok(())
    }
}
