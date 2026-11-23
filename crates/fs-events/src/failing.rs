//! `FailingAppointments`: a provider that looks capable and answers every call with `Unavailable` (Z3).
//!
//! Pattern: Null Object variant (the "broken" one), the trait's every method is overridden.

use async_trait::async_trait;

use crate::draft::{
    Answer, Appointment, AppointmentDraft, AppointmentPatch, AppointmentQuery, AppointmentSummary,
    Appointments, Attendee, Availability, CalendarDeleted, CalendarInfo, CalendarPatch,
    Capabilities, Change, Deleted, Error, HistoryEntry, Invitee, ObjectId, Page, PersonRef, Place,
    Reference, Refresh, Refreshed, Result, Revision, SeriesScope, Trash, Utc, When, Window, Zone,
};

/// Every method fails with `Error::Unavailable(reason)`.
#[derive(Clone, Debug)]
pub struct FailingAppointments {
    reason: String,
}

impl FailingAppointments {
    /// A failing provider with the given reason.
    #[must_use]
    pub fn new(reason: &str) -> Self {
        Self {
            reason: reason.to_owned(),
        }
    }

    fn fail<T>(&self) -> Result<T> {
        Err(Error::Unavailable(self.reason.clone()))
    }
}

impl Default for FailingAppointments {
    fn default() -> Self {
        Self::new("store not reachable")
    }
}

#[async_trait]
impl Appointments for FailingAppointments {
    fn capabilities(&self) -> Capabilities {
        Capabilities(
            crate::draft::CAPABILITIES
                .iter()
                .map(|c| ((*c).to_owned(), true))
                .collect(),
        )
    }
    async fn calendars(&self, _: Trash) -> Result<Vec<CalendarInfo>> {
        self.fail()
    }
    async fn create_calendar(
        &self,
        _: &str,
        _: Option<&str>,
        _: Option<Zone>,
        _: Option<PersonRef>,
    ) -> Result<CalendarInfo> {
        self.fail()
    }
    async fn update_calendar(
        &self,
        _: ObjectId,
        _: Revision,
        _: CalendarPatch,
    ) -> Result<CalendarInfo> {
        self.fail()
    }
    async fn delete_calendar(&self, _: ObjectId, _: Revision) -> Result<CalendarDeleted> {
        self.fail()
    }
    async fn restore_calendar(&self, _: ObjectId) -> Result<CalendarInfo> {
        self.fail()
    }
    async fn subscribe_calendar(
        &self,
        _: &str,
        _: Option<&str>,
        _: &Reference,
        _: Refresh,
    ) -> Result<CalendarInfo> {
        self.fail()
    }
    async fn refresh_calendar(&self, _: ObjectId) -> Result<Refreshed> {
        self.fail()
    }
    async fn purge(&self, _: ObjectId) -> Result<()> {
        self.fail()
    }
    async fn get(&self, _: ObjectId) -> Result<Option<Appointment>> {
        self.fail()
    }
    async fn get_by_uid(&self, _: &str) -> Result<Option<Appointment>> {
        self.fail()
    }
    async fn list(
        &self,
        _: &AppointmentQuery,
        _: Option<&Place>,
        _: usize,
    ) -> Result<Page<AppointmentSummary>> {
        self.fail()
    }
    async fn window(&self, _: Utc, _: Utc, _: &Zone, _: &[ObjectId]) -> Result<Window> {
        self.fail()
    }
    async fn search(
        &self,
        _: &str,
        _: Option<&Place>,
        _: usize,
    ) -> Result<Page<AppointmentSummary>> {
        self.fail()
    }
    async fn create(&self, _: AppointmentDraft) -> Result<Appointment> {
        self.fail()
    }
    async fn update(
        &self,
        _: ObjectId,
        _: Revision,
        _: AppointmentPatch,
        _: SeriesScope,
    ) -> Result<Appointment> {
        self.fail()
    }
    async fn delete(&self, _: ObjectId, _: Revision, _: SeriesScope) -> Result<Deleted> {
        self.fail()
    }
    async fn restore(&self, _: ObjectId, _: SeriesScope) -> Result<Appointment> {
        self.fail()
    }
    async fn invite(
        &self,
        _: ObjectId,
        _: Revision,
        _: &[Invitee],
        _: SeriesScope,
    ) -> Result<Appointment> {
        self.fail()
    }
    async fn respond(
        &self,
        _: ObjectId,
        _: Revision,
        _: &PersonRef,
        _: Answer,
        _: Option<&str>,
        _: Option<When>,
        _: SeriesScope,
    ) -> Result<Appointment> {
        self.fail()
    }
    async fn free_busy(
        &self,
        _: &[Attendee],
        _: Utc,
        _: Utc,
        _: &Zone,
    ) -> Result<Vec<Availability>> {
        self.fail()
    }
    async fn history(
        &self,
        _: ObjectId,
        _: Option<&Place>,
        _: usize,
    ) -> Result<Page<HistoryEntry>> {
        self.fail()
    }
    async fn changes(&self, _: Option<&Place>, _: usize) -> Result<Page<Change>> {
        self.fail()
    }
}
