//! A provider that forwards everything to a `FakeAppointments` and breaks ONE rule on purpose (a mutant), to show
//! that the conformance rules notice it.

use std::sync::Arc;

use crate::mutant::{Mutant, is};
use async_trait::async_trait;
use fs_calendars_testdata::FakeAppointments;
use fs_calendars_testdata::draft::{
    Answer, Appointment, AppointmentDraft, AppointmentPatch, AppointmentQuery, AppointmentSummary,
    Appointments, Attendee, Availability, CalendarDeleted, CalendarInfo, CalendarPatch,
    Capabilities, Change, Deleted, Error, FreeBusyAnswer, HistoryEntry, Invitee, ObjectId, Page,
    PersonRef, Place, Reference, Refresh, Refreshed, Result, Revision, SeriesScope, Trash, Utc,
    When, Window, Zone,
};

/// The forwarding provider.
pub struct Wrap(pub Arc<FakeAppointments>, pub Mutant);

#[async_trait]
impl Appointments for Wrap {
    fn capabilities(&self) -> Capabilities {
        self.0.capabilities()
    }
    async fn calendars(&self, trash: Trash) -> Result<Vec<CalendarInfo>> {
        crate::tamper::calendars(self.1, self.0.calendars(trash).await)
    }
    async fn create_calendar(
        &self,
        name: &str,
        color: Option<&str>,
        zone: Option<Zone>,
        owner: Option<PersonRef>,
    ) -> Result<CalendarInfo> {
        let r = self.0.create_calendar(name, color, zone, owner).await;
        match r {
            Err(Error::Forbidden { .. }) if is(self.1, Mutant::AiOwnerInvalid) => {
                Err(Error::Invalid(Vec::new()))
            }
            other => other,
        }
    }
    async fn update_calendar(
        &self,
        id: ObjectId,
        expected: Revision,
        change: CalendarPatch,
    ) -> Result<CalendarInfo> {
        if is(self.1, Mutant::NoCompareAndSet) {
            let all = self.0.calendars(Trash::Include).await?;
            let current = all
                .iter()
                .find(|c| c.id == id)
                .map_or(expected, |c| c.revision);
            return self.0.update_calendar(id, current, change).await;
        }
        let r = self.0.update_calendar(id, expected, change).await;
        match r {
            Err(Error::NotFound(_)) if is(self.1, Mutant::HiddenForbidden) => {
                Err(Error::Forbidden {
                    capability: "calendar.update".to_owned(),
                })
            }
            other => other,
        }
    }
    async fn delete_calendar(&self, id: ObjectId, expected: Revision) -> Result<CalendarDeleted> {
        let mut r = crate::tamper::forbidden_as_invalid(
            is(self.1, Mutant::AiDeleteInvalid),
            self.0.delete_calendar(id, expected).await,
        );
        if let (Ok(d), true) = (&mut r, is(self.1, Mutant::CountLost)) {
            d.appointments = 0;
        }
        r
    }
    async fn restore_calendar(&self, id: ObjectId) -> Result<CalendarInfo> {
        if crate::tamper::held_not_restorable(self.1, &self.0, id).await {
            return Err(fs_calendars_testdata::draft::Error::NotFound(id));
        }
        let r = self.0.restore_calendar(id).await;
        crate::tamper::restore_calendar(self.1, &self.0, r).await
    }
    async fn subscribe_calendar(
        &self,
        name: &str,
        color: Option<&str>,
        url0: &Reference,
        refresh: Refresh,
    ) -> Result<CalendarInfo> {
        let url = crate::tamper::feed(self.1, url0);
        let r = self
            .0
            .subscribe_calendar(name, color, url.as_ref().unwrap_or(url0), refresh)
            .await;
        crate::tamper::forbidden_as_invalid(is(self.1, Mutant::AiSubscribeInvalid), r)
    }
    async fn refresh_calendar(&self, id: ObjectId) -> Result<Refreshed> {
        self.0.refresh_calendar(id).await
    }
    async fn get(&self, id: ObjectId) -> Result<Option<Appointment>> {
        let mut r = self.0.get(id).await;
        if let (Ok(Some(a)), true) = (&mut r, is(self.1, Mutant::FeedInOrigin)) {
            if let Some(o) = &mut a.origin {
                o.source = fs_calendars_testdata::zone::reference(
                    "https://kalender.example.org/zzconf-feed.ics",
                );
            }
        }
        r
    }
    async fn get_by_uid(&self, uid: &str) -> Result<Option<Appointment>> {
        self.0.get_by_uid(uid).await
    }
    async fn list(
        &self,
        q: &AppointmentQuery,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<AppointmentSummary>> {
        let mut r = self.0.list(q, after, size).await;
        if let Ok(p) = &mut r {
            crate::tamper::trash_view(self.1, &mut p.items);
            if is(self.1, Mutant::HeldCalendarLosesAppointments) {
                let held = self.0.calendars(Trash::Only).await.is_ok_and(|l| {
                    l.iter().any(|c| {
                        matches!(
                            c.trash,
                            Some(fs_calendars_testdata::draft::Restorable::Held(_))
                        )
                    })
                });
                if held {
                    p.items.retain(|s| s.trash.is_none());
                }
            }
        }
        r
    }
    async fn window(
        &self,
        from: Utc,
        to: Utc,
        view_zone: &Zone,
        calendars: &[ObjectId],
    ) -> Result<Window> {
        self.0.window(from, to, view_zone, calendars).await
    }
    async fn search(
        &self,
        text: &str,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<AppointmentSummary>> {
        self.0.search(text, after, size).await
    }
    async fn create(&self, draft: AppointmentDraft) -> Result<Appointment> {
        let r = crate::tamper::create(self.1, &self.0, draft).await;
        match r {
            Err(Error::NotFound(_)) if is(self.1, Mutant::TrashedWriteInvalid) => {
                Err(Error::Invalid(Vec::new()))
            }
            Err(Error::ReadOnly { .. }) if is(self.1, Mutant::ReadOnlyAsForbidden) => {
                Err(Error::Forbidden {
                    capability: "event.create".to_owned(),
                })
            }
            other => other,
        }
    }
    async fn update(
        &self,
        id: ObjectId,
        expected: Revision,
        change: AppointmentPatch,
        scope: SeriesScope,
    ) -> Result<Appointment> {
        let r = self.0.update(id, expected, change, scope).await;
        match r {
            Err(Error::NotFound(_)) if is(self.1, Mutant::TrashedWriteInvalid) => {
                Err(Error::Invalid(Vec::new()))
            }
            Err(Error::ReadOnly { .. }) if is(self.1, Mutant::ReadOnlyAsForbidden) => {
                Err(Error::Forbidden {
                    capability: "event.update".to_owned(),
                })
            }
            other => other,
        }
    }
    async fn delete(
        &self,
        id: ObjectId,
        expected: Revision,
        scope: SeriesScope,
    ) -> Result<Deleted> {
        self.0.delete(id, expected, scope).await
    }
    async fn restore(&self, id: ObjectId, scope: SeriesScope) -> Result<Appointment> {
        let r = self.0.restore(id, scope).await;
        match r {
            Err(Error::ReadOnly { .. }) if is(self.1, Mutant::RestoreSubscribedForbidden) => {
                Err(Error::Forbidden {
                    capability: "event.restore".to_owned(),
                })
            }
            other => other,
        }
    }
    async fn purge(&self, id: ObjectId) -> Result<()> {
        match self.1 {
            Mutant::PurgeIgnoresHold => {
                self.0.release(id);
                self.0.purge(id).await
            }
            Mutant::PurgeDoesNothing => Ok(()),
            Mutant::PurgeLivingOk => match self.0.purge(id).await {
                Err(Error::Invalid(_)) => Ok(()),
                other => other,
            },
            m => crate::tamper::forbidden_as_invalid(
                is(m, Mutant::AiPurgeInvalid),
                self.0.purge(id).await,
            ),
        }
    }
    async fn invite(
        &self,
        id: ObjectId,
        expected: Revision,
        invitees: &[Invitee],
        scope: SeriesScope,
    ) -> Result<Appointment> {
        self.0.invite(id, expected, invitees, scope).await
    }
    async fn respond(
        &self,
        id: ObjectId,
        expected: Revision,
        who: &PersonRef,
        answer: Answer,
        comment: Option<&str>,
        proposal: Option<When>,
        scope: SeriesScope,
    ) -> Result<Appointment> {
        self.0
            .respond(id, expected, who, answer, comment, proposal, scope)
            .await
    }
    async fn free_busy(
        &self,
        who: &[Attendee],
        from: Utc,
        to: Utc,
        view_zone: &Zone,
    ) -> Result<Vec<Availability>> {
        let mut r = self.0.free_busy(who, from, to, view_zone).await;
        if let Ok(list) = &mut r {
            for a in list {
                match (&a.answer, self.1) {
                    (FreeBusyAnswer::NotShared, Mutant::NotSharedAsUnknown) => {
                        a.answer = FreeBusyAnswer::Unknown;
                    }
                    (FreeBusyAnswer::Unknown, Mutant::UnknownGuessedFree) => {
                        a.answer = FreeBusyAnswer::Busy(Vec::new());
                    }
                    _ => {}
                }
            }
        }
        r
    }
    async fn history(
        &self,
        id: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<HistoryEntry>> {
        self.0.history(id, after, size).await
    }
    async fn changes(&self, since: Option<&Place>, size: usize) -> Result<Page<Change>> {
        self.0.changes(since, size).await
    }
}
