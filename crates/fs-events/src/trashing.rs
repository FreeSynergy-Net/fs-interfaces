//! The trash rules of the fake: the period, the retention, the sweep and what a calendar shows of itself.
//!
//! Pattern: Repository state helpers (pure functions of the state; the period and the holds are settings of the
//! program, the trait only shows `Restorable`).

use crate::draft::{
    CalendarInfo, CalendarSource, Change, Error, FieldChange, HistoryEntry, HistoryKind, ObjectId,
    Restorable, Result, Subscription, Utc,
};
use crate::state::{Item, State};

impl State {
    /// The retention that applies to these objects now: `Some(None)` = without end, `Some(Some(t))` = until the
    /// latest `t`, `None` = none (a retention that has ended does not count).
    #[must_use]
    pub fn active_hold(&self, ids: &[ObjectId]) -> Option<Option<Utc>> {
        let mut best: Option<Option<Utc>> = None;
        for end in ids.iter().filter_map(|i| self.holds.get(i)) {
            match end {
                None => return Some(None),
                Some(t) if *t > self.now => {
                    best = Some(Some(best.flatten().map_or(*t, |b| b.max(*t))));
                }
                Some(_) => {}
            }
        }
        best
    }

    /// What `restorable` says for something deleted at `at`: the later of period and retention (K8: with "never
    /// automatically" and a retention WITH an end, `Held(Some(end))`; the retention goes before "never").
    #[must_use]
    pub fn restorable(&self, ids: &[ObjectId], at: Utc) -> Restorable {
        let frame = self.period_ms.map(|p| Utc::from_millis(at.millis() + p));
        match self.active_hold(ids) {
            Some(None) => Restorable::Held(None),
            Some(Some(t)) => Restorable::Held(Some(frame.map_or(t, |f| f.max(t)))),
            None => frame.map_or(Restorable::Manual, Restorable::Until),
        }
    }

    fn expired(&self, ids: &[ObjectId], at: Utc) -> bool {
        self.period_ms
            .is_some_and(|p| self.now.millis() >= at.millis() + p)
            && self.active_hold(ids).is_none()
    }

    /// The trash state of an appointment: `None` while it lives.
    #[must_use]
    pub fn item_trash(&self, item: &Item) -> Option<Restorable> {
        let at = item.deleted?;
        let mut ids = vec![item.appt.id];
        if item.via_calendar {
            ids.push(item.appt.calendar);
        }
        Some(self.restorable(&ids, at))
    }

    /// The ids whose retention counts for a calendar in the trash: itself and everything trashed in it.
    #[must_use]
    pub fn calendar_ids(&self, cal: ObjectId) -> Vec<ObjectId> {
        let mut ids = vec![cal];
        ids.extend(
            self.items
                .values()
                .filter(|i| i.appt.calendar == cal && i.deleted.is_some())
                .map(|i| i.appt.id),
        );
        ids
    }

    /// Remove what the period has run out for (the nightly sweep of a real provider), with its log.
    pub fn sweep(&mut self) {
        let cals: Vec<ObjectId> = self
            .cal_deleted
            .iter()
            .filter(|(c, at)| self.expired(&self.calendar_ids(**c), **at))
            .map(|(c, _)| *c)
            .collect();
        for c in cals {
            self.remove_calendar(c);
        }
        let items: Vec<ObjectId> = self
            .items
            .values()
            .filter(|i| !i.via_calendar)
            .filter(|i| i.deleted.is_some_and(|at| self.expired(&[i.appt.id], at)))
            .map(|i| i.appt.id)
            .collect();
        for id in items {
            self.remove_item(id);
        }
    }

    /// Delete an appointment for good, with its log.
    pub fn remove_item(&mut self, id: ObjectId) {
        self.items.remove(&id);
        self.history.retain(|(i, _)| *i != id);
        self.cuts.retain(|c| c.id != id);
        self.holds.remove(&id);
    }

    /// Delete a calendar for good, with everything in it and the log of all of it.
    pub fn remove_calendar(&mut self, cal: ObjectId) {
        let ids: Vec<ObjectId> = self
            .items
            .values()
            .filter(|i| i.appt.calendar == cal)
            .map(|i| i.appt.id)
            .collect();
        for id in ids {
            self.remove_item(id);
        }
        self.calendars.retain(|c| c.id != cal);
        self.cal_deleted.remove(&cal);
        self.history.retain(|(i, _)| *i != cal);
        self.holds.remove(&cal);
    }

    /// The calendar as the asker sees it: its trash state, and the address of a feed or account only for who may
    /// change the calendar (here: its owner).
    #[must_use]
    pub fn calendar_view(&self, c: &CalendarInfo) -> CalendarInfo {
        let mut v = c.clone();
        v.trash = self
            .cal_deleted
            .get(&c.id)
            .map(|at| self.restorable(&self.calendar_ids(c.id), *at));
        if c.owner != self.viewer {
            match &mut v.source {
                CalendarSource::Subscribed(Subscription { url, .. }) => *url = None,
                CalendarSource::Account { collection, .. } => *collection = None,
                CalendarSource::Own => {}
            }
        }
        v
    }

    /// A calendar the asker may see, living or in the trash.
    ///
    /// # Errors
    /// `NotFound` for an unknown calendar or one hidden from an asking AI.
    pub fn calendar(&self, id: ObjectId) -> Result<&CalendarInfo> {
        self.calendars
            .iter()
            .find(|c| c.id == id && self.may_see(id))
            .ok_or(Error::NotFound(id))
    }

    /// A calendar a write may go to: living, visible, not read-only.
    ///
    /// # Errors
    /// `NotFound` (also when in the trash), `ReadOnly` for a subscription.
    pub fn writable_calendar(&self, id: ObjectId) -> Result<&CalendarInfo> {
        let c = self.calendar(id)?;
        if self.cal_deleted.contains_key(&id) {
            return Err(Error::NotFound(id));
        }
        if matches!(c.source, CalendarSource::Subscribed(_)) {
            return Err(Error::ReadOnly { calendar: id });
        }
        Ok(c)
    }

    /// One entry in the log of a calendar and one in the change feed (`appointment = None`).
    pub fn record_calendar(&mut self, cal: ObjectId, kind: HistoryKind, changes: Vec<FieldChange>) {
        let (at, actor) = (self.now, self.viewer.clone());
        self.history.push((
            cal,
            HistoryEntry {
                at,
                actor,
                kind,
                changes,
            },
        ));
        self.changes.push(Change {
            appointment: None,
            calendar: cal,
            kind,
            at,
        });
    }
}
