//! `SlowAppointments`: any provider, every call delayed (Z2: the grid shows its loading state).
//!
//! Pattern: Decorator over `Appointments`; the delay is awaited with `tokio::time::sleep`, so a test can use
//! paused time.

use std::time::Duration;

use async_trait::async_trait;

use crate::draft::{
    Answer, Appointment, AppointmentDraft, AppointmentPatch, AppointmentQuery, AppointmentSummary,
    Appointments, Attendee, Availability, CalendarDeleted, CalendarInfo, CalendarPatch,
    Capabilities, Change, Deleted, HistoryEntry, Invitee, ObjectId, Page, PersonRef, Place,
    Reference, Refresh, Refreshed, Result, Revision, SeriesScope, Trash, Utc, When, Window, Zone,
};

/// `inner` with `delay` before every answer.
#[derive(Debug)]
pub struct SlowAppointments<A> {
    inner: A,
    delay: Duration,
}

impl<A: Appointments> SlowAppointments<A> {
    /// Wrap `inner`; the plan's Z2 uses three seconds.
    #[must_use]
    pub fn new(inner: A, delay: Duration) -> Self {
        Self { inner, delay }
    }

    async fn wait(&self) {
        tokio::time::sleep(self.delay).await;
    }
}

#[async_trait]
impl<A: Appointments> Appointments for SlowAppointments<A> {
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    async fn calendars(&self, trash: Trash) -> Result<Vec<CalendarInfo>> {
        self.wait().await;
        self.inner.calendars(trash).await
    }
    async fn create_calendar(
        &self,
        name: &str,
        color: Option<&str>,
        zone: Option<Zone>,
        owner: Option<PersonRef>,
    ) -> Result<CalendarInfo> {
        self.wait().await;
        self.inner.create_calendar(name, color, zone, owner).await
    }
    async fn update_calendar(
        &self,
        id: ObjectId,
        expected: Revision,
        change: CalendarPatch,
    ) -> Result<CalendarInfo> {
        self.wait().await;
        self.inner.update_calendar(id, expected, change).await
    }
    async fn delete_calendar(&self, id: ObjectId, expected: Revision) -> Result<CalendarDeleted> {
        self.wait().await;
        self.inner.delete_calendar(id, expected).await
    }
    async fn restore_calendar(&self, id: ObjectId) -> Result<CalendarInfo> {
        self.wait().await;
        self.inner.restore_calendar(id).await
    }
    async fn subscribe_calendar(
        &self,
        name: &str,
        color: Option<&str>,
        url: &Reference,
        refresh: Refresh,
    ) -> Result<CalendarInfo> {
        self.wait().await;
        self.inner
            .subscribe_calendar(name, color, url, refresh)
            .await
    }
    async fn refresh_calendar(&self, id: ObjectId) -> Result<Refreshed> {
        self.wait().await;
        self.inner.refresh_calendar(id).await
    }
    async fn purge(&self, id: ObjectId) -> Result<()> {
        self.wait().await;
        self.inner.purge(id).await
    }
    async fn get(&self, id: ObjectId) -> Result<Option<Appointment>> {
        self.wait().await;
        self.inner.get(id).await
    }
    async fn get_by_uid(&self, uid: &str) -> Result<Option<Appointment>> {
        self.wait().await;
        self.inner.get_by_uid(uid).await
    }
    async fn list(
        &self,
        q: &AppointmentQuery,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<AppointmentSummary>> {
        self.wait().await;
        self.inner.list(q, after, size).await
    }
    async fn window(
        &self,
        from: Utc,
        to: Utc,
        view_zone: &Zone,
        calendars: &[ObjectId],
    ) -> Result<Window> {
        self.wait().await;
        self.inner.window(from, to, view_zone, calendars).await
    }
    async fn search(
        &self,
        text: &str,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<AppointmentSummary>> {
        self.wait().await;
        self.inner.search(text, after, size).await
    }
    async fn create(&self, draft: AppointmentDraft) -> Result<Appointment> {
        self.wait().await;
        self.inner.create(draft).await
    }
    async fn update(
        &self,
        id: ObjectId,
        expected: Revision,
        change: AppointmentPatch,
        scope: SeriesScope,
    ) -> Result<Appointment> {
        self.wait().await;
        self.inner.update(id, expected, change, scope).await
    }
    async fn delete(
        &self,
        id: ObjectId,
        expected: Revision,
        scope: SeriesScope,
    ) -> Result<Deleted> {
        self.wait().await;
        self.inner.delete(id, expected, scope).await
    }
    async fn restore(&self, id: ObjectId, scope: SeriesScope) -> Result<Appointment> {
        self.wait().await;
        self.inner.restore(id, scope).await
    }
    async fn invite(
        &self,
        id: ObjectId,
        expected: Revision,
        invitees: &[Invitee],
        scope: SeriesScope,
    ) -> Result<Appointment> {
        self.wait().await;
        self.inner.invite(id, expected, invitees, scope).await
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
        self.wait().await;
        self.inner
            .respond(id, expected, who, answer, comment, proposal, scope)
            .await
    }
    async fn free_busy(
        &self,
        people: &[Attendee],
        from: Utc,
        to: Utc,
        view_zone: &Zone,
    ) -> Result<Vec<Availability>> {
        self.wait().await;
        self.inner.free_busy(people, from, to, view_zone).await
    }
    async fn history(
        &self,
        id: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<HistoryEntry>> {
        self.wait().await;
        self.inner.history(id, after, size).await
    }
    async fn changes(&self, since: Option<&Place>, size: usize) -> Result<Page<Change>> {
        self.wait().await;
        self.inner.changes(since, size).await
    }
}
