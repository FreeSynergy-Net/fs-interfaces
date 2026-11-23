//! Creating calendars and appointments, changing, deleting and restoring (thin over [`crate::scope`]).
//!
//! Pattern: Facade over the command executor in `scope.rs`; this file only builds and checks the new thing.

use crate::check;
use crate::draft::{
    Appointment, AppointmentDraft, AppointmentPatch, Deleted, Error, HistoryKind, ObjectId,
    Problem, Result, Revision, SeriesScope,
};
use crate::fake::FakeAppointments;
use crate::patch::{apply, describe};
use crate::scope;
use crate::state::{Item, blank};
use crate::trash;

/// A new appointment has no recurrence yet: any `Original` rule in the draft is a new one.
fn blank_like(draft: &AppointmentDraft) -> Appointment {
    blank(
        crate::ids::object_id(0),
        draft.calendar,
        String::new(),
        &draft.title,
        draft.when.clone(),
        crate::draft::Utc::from_millis(0),
    )
}

impl FakeAppointments {
    pub(crate) fn create_impl(&self, draft: AppointmentDraft) -> Result<Appointment> {
        let mut s = self.lock();
        s.writable_calendar(draft.calendar)?;
        crate::scope::no_new_original(&blank_like(&draft), &draft.rest, "event.create")?;
        let id = s.fresh_id();
        let uid = draft.uid.unwrap_or_else(|| {
            format!(
                "urn:uuid:00000000-0000-7000-9000-{:012x}",
                crate::ids::object_number(id)
            )
        });
        if s.items.values().any(|i| i.appt.uid == uid) {
            return Err(Error::Invalid(vec![Problem {
                field: "uid".to_owned(),
                reason: "already exists".to_owned(),
            }]));
        }
        let mut a = blank(id, draft.calendar, uid, &draft.title, draft.when, s.now);
        apply(&mut a, &draft.rest);
        check::appointment(&a, &s.customs)?;
        let changes = describe(
            &blank(id, draft.calendar, a.uid.clone(), "", a.when.clone(), s.now),
            &AppointmentPatch {
                title: Some(a.title.clone()),
                ..draft.rest
            },
        );
        s.record(&a, HistoryKind::Create, changes);
        s.items.insert(
            id,
            Item {
                appt: a.clone(),
                deleted: None,
                via_calendar: false,
            },
        );
        Ok(a)
    }

    pub(crate) fn update_impl(
        &self,
        id: ObjectId,
        expected: Revision,
        change: &AppointmentPatch,
        scope: SeriesScope,
    ) -> Result<Appointment> {
        scope::write(
            &mut self.lock(),
            id,
            Some(expected),
            change,
            scope,
            HistoryKind::Update,
            Vec::new(),
        )
    }

    pub(crate) fn delete_impl(
        &self,
        id: ObjectId,
        expected: Revision,
        scope: SeriesScope,
    ) -> Result<Deleted> {
        trash::delete(&mut self.lock(), id, expected, scope)
    }

    pub(crate) fn restore_impl(&self, id: ObjectId, scope: SeriesScope) -> Result<Appointment> {
        trash::restore(&mut self.lock(), id, scope)
    }
}
