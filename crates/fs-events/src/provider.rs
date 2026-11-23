//! `impl Appointments for FakeAppointments`: one line per method, the work is in the other files.
//!
//! Pattern: Adapter (trait to the inherent methods).

use async_trait::async_trait;

use crate::draft::{
    Answer, Appointment, AppointmentDraft, AppointmentPatch, AppointmentQuery, AppointmentSummary,
    Appointments, Attendee, Availability, CAPABILITIES, CalendarDeleted, CalendarInfo,
    CalendarPatch, Capabilities, Change, Deleted, HistoryEntry, Invitee, ObjectId, Page, PersonRef,
    Place, Reference, Refresh, Refreshed, Result, Revision, SeriesScope, Trash, Utc, When, Window,
    Zone,
};
use crate::fake::FakeAppointments;

/// The capability ids a fake offers: all of them.
#[must_use]
pub fn fake_capabilities() -> Capabilities {
    Capabilities(
        CAPABILITIES
            .iter()
            .map(|c| ((*c).to_owned(), true))
            .collect(),
    )
}

#[async_trait]
impl Appointments for FakeAppointments {
    fn capabilities(&self) -> Capabilities {
        fake_capabilities()
    }
    async fn calendars(&self, trash: Trash) -> Result<Vec<CalendarInfo>> {
        Ok(self.calendars_impl(trash))
    }
    async fn create_calendar(
        &self,
        name: &str,
        color: Option<&str>,
        default_zone: Option<Zone>,
        owner: Option<PersonRef>,
    ) -> Result<CalendarInfo> {
        self.create_calendar_impl(name, color, default_zone, owner)
    }
    async fn update_calendar(
        &self,
        id: ObjectId,
        expected: Revision,
        change: CalendarPatch,
    ) -> Result<CalendarInfo> {
        self.update_calendar_impl(id, expected, &change)
    }
    async fn delete_calendar(&self, id: ObjectId, expected: Revision) -> Result<CalendarDeleted> {
        self.delete_calendar_impl(id, expected)
    }
    async fn restore_calendar(&self, id: ObjectId) -> Result<CalendarInfo> {
        self.restore_calendar_impl(id)
    }
    async fn subscribe_calendar(
        &self,
        name: &str,
        color: Option<&str>,
        url: &Reference,
        refresh: Refresh,
    ) -> Result<CalendarInfo> {
        self.subscribe_calendar_impl(name, color, url, refresh)
    }
    async fn refresh_calendar(&self, id: ObjectId) -> Result<Refreshed> {
        self.refresh_calendar_impl(id)
    }
    async fn get(&self, id: ObjectId) -> Result<Option<Appointment>> {
        Ok(self.get_impl(id))
    }
    async fn get_by_uid(&self, uid: &str) -> Result<Option<Appointment>> {
        Ok(self.get_by_uid_impl(uid))
    }
    async fn list(
        &self,
        query: &AppointmentQuery,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<AppointmentSummary>> {
        self.list_impl(query, after, size)
    }
    async fn window(
        &self,
        from: Utc,
        to: Utc,
        view_zone: &Zone,
        calendars: &[ObjectId],
    ) -> Result<Window> {
        self.window_impl(from, to, view_zone, calendars)
    }
    async fn search(
        &self,
        text: &str,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<AppointmentSummary>> {
        self.search_impl(text, after, size)
    }
    async fn create(&self, draft: AppointmentDraft) -> Result<Appointment> {
        self.create_impl(draft)
    }
    async fn update(
        &self,
        id: ObjectId,
        expected: Revision,
        change: AppointmentPatch,
        scope: SeriesScope,
    ) -> Result<Appointment> {
        self.update_impl(id, expected, &change, scope)
    }
    async fn delete(
        &self,
        id: ObjectId,
        expected: Revision,
        scope: SeriesScope,
    ) -> Result<Deleted> {
        self.delete_impl(id, expected, scope)
    }
    async fn restore(&self, id: ObjectId, scope: SeriesScope) -> Result<Appointment> {
        self.restore_impl(id, scope)
    }
    async fn invite(
        &self,
        id: ObjectId,
        expected: Revision,
        invitees: &[Invitee],
        scope: SeriesScope,
    ) -> Result<Appointment> {
        self.invite_impl(id, expected, invitees, scope)
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
        self.respond_impl(id, expected, who, answer, comment, proposal.as_ref(), scope)
    }
    async fn purge(&self, id: ObjectId) -> Result<()> {
        self.purge_impl(id)
    }
    async fn free_busy(
        &self,
        who: &[Attendee],
        from: Utc,
        to: Utc,
        view_zone: &Zone,
    ) -> Result<Vec<Availability>> {
        self.free_busy_impl(who, from, to, view_zone)
    }
    async fn history(
        &self,
        id: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<HistoryEntry>> {
        self.history_impl(id, after, size)
    }
    async fn changes(&self, since: Option<&Place>, size: usize) -> Result<Page<Change>> {
        self.changes_impl(since, size)
    }
}
