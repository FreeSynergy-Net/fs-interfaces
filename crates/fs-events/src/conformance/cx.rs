//! The context of a conformance run: the provider, the report and the small helpers the rules share.
//!
//! Pattern: Test helper (plain functions; a setup that fails is a failure of the rule "setup", never a panic).

use super::fixture::{Advance, ConformanceFixture};
use crate::draft::{
    Appointment, AppointmentDraft, AppointmentPatch, AppointmentQuery, AppointmentSummary,
    Appointments, CalendarInfo, Error, ObjectId, Result, Trash, When, Zone,
};
use crate::sample::{at, timed};
use crate::zone::iana;

/// The failed and the skipped rules of a run; empty `failures` means conformant.
#[derive(Debug, Default)]
pub struct Report {
    /// Rules that failed.
    pub failures: Vec<String>,
    /// Rules a fixture could not set up (it has no such mutator); not a failure, but not proven either.
    pub skipped: Vec<String>,
    /// How many checks ran (a rule that returned early because its setup failed is not counted).
    pub checked: usize,
}

/// Provider, report and zone of one run.
pub(crate) struct Cx<'a> {
    pub fx: &'a dyn ConformanceFixture,
    pub dir: &'a dyn Appointments,
    pub report: Report,
    pub berlin: Zone,
}

impl<'a> Cx<'a> {
    pub(crate) fn new(fx: &'a dyn ConformanceFixture) -> Self {
        Self {
            fx,
            dir: fx.appointments(),
            report: Report::default(),
            berlin: iana("Europe/Berlin"),
        }
    }

    pub(crate) fn ensure(&mut self, ok: bool, rule: &str) {
        self.report.checked += 1;
        if !ok {
            self.report.failures.push(rule.to_owned());
        }
    }

    pub(crate) fn skip(&mut self, rule: &str) {
        self.report.skipped.push(rule.to_owned());
    }

    /// Move the clock: `true` when moved; not offered = skipped (named by `rule`); a failed move is a failure.
    pub(crate) async fn advance(&mut self, millis: i64, rule: &str) -> bool {
        match self.fx.advance_clock(millis).await {
            Advance::Done => true,
            Advance::Unsupported => {
                self.skip(&format!("{rule} (the fixture cannot move the clock)"));
                false
            }
            Advance::Failed(why) => {
                self.ensure(false, &format!("{rule}: the clock move failed: {why}"));
                false
            }
        }
    }

    /// The call must fail, and the error must be the wanted one.
    pub(crate) fn fails<T>(
        &mut self,
        result: &Result<T>,
        rule: &str,
        want: impl Fn(&Error) -> bool,
    ) {
        let ok = result.as_ref().err().is_some_and(want);
        self.ensure(ok, rule);
    }

    /// Whether the provider offers all of these capabilities.
    pub(crate) fn offers(&self, names: &[&str]) -> bool {
        let caps = self.dir.capabilities().0;
        names
            .iter()
            .all(|n| caps.iter().any(|(c, on)| c == n && *on))
    }

    /// A calendar named `Zzconf <name>` owned by the asker.
    pub(crate) async fn calendar(&mut self, name: &str) -> Option<CalendarInfo> {
        let made = self
            .dir
            .create_calendar(&format!("Zzconf {name}"), None, None, None)
            .await;
        self.ensure(made.is_ok(), &format!("setup: create_calendar {name}"));
        made.ok()
    }

    /// The time of the appointments the rules make: `day` of March 2097 (far away from any real data).
    pub(crate) fn when(day: u8) -> When {
        timed(at(2097, 3, day, 9, 0), "Europe/Berlin", 60)
    }

    /// An appointment `Zzconf <title>` on `day` of March 2097.
    pub(crate) async fn appointment(
        &mut self,
        cal: ObjectId,
        title: &str,
        day: u8,
    ) -> Option<Appointment> {
        self.appointment_at(cal, title, Self::when(day)).await
    }

    /// An appointment `Zzconf <title>` at this time.
    pub(crate) async fn appointment_at(
        &mut self,
        cal: ObjectId,
        title: &str,
        when: When,
    ) -> Option<Appointment> {
        let draft = AppointmentDraft {
            calendar: cal,
            uid: None,
            title: format!("Zzconf {title}"),
            when,
            rest: AppointmentPatch::default(),
        };
        let made = self.dir.create(draft).await;
        self.ensure(made.is_ok(), &format!("setup: create {title}"));
        made.ok()
    }

    /// The appointment as stored now (`None` when it is gone or in the trash).
    pub(crate) async fn fresh(&self, id: ObjectId) -> Option<Appointment> {
        self.dir.get(id).await.ok().flatten()
    }

    /// The calendar as listed now, living or in the trash.
    pub(crate) async fn listed(&self, id: ObjectId) -> Option<CalendarInfo> {
        let all = self.dir.calendars(Trash::Include).await.ok()?;
        all.into_iter().find(|c| c.id == id)
    }

    /// All summaries of a query over these calendars (one big page).
    pub(crate) async fn all(&self, cals: &[ObjectId], trash: Trash) -> Vec<AppointmentSummary> {
        let q = AppointmentQuery {
            calendars: cals.to_vec(),
            trash,
            ..AppointmentQuery::default()
        };
        self.dir
            .list(&q, None, 100_000)
            .await
            .map(|p| p.items)
            .unwrap_or_default()
    }
}

pub(crate) fn is_invalid(e: &Error) -> bool {
    matches!(e, Error::Invalid(_))
}
pub(crate) fn is_not_found(e: &Error) -> bool {
    matches!(e, Error::NotFound(_))
}
pub(crate) fn is_changed(e: &Error) -> bool {
    matches!(e, Error::Changed { .. })
}
pub(crate) fn not_found_of(id: ObjectId) -> impl Fn(&Error) -> bool {
    move |e| matches!(e, Error::NotFound(x) if *x == id)
}
pub(crate) fn read_only_of(id: ObjectId) -> impl Fn(&Error) -> bool {
    move |e| matches!(e, Error::ReadOnly { calendar } if *calendar == id)
}
pub(crate) fn forbidden(capability: &'static str) -> impl Fn(&Error) -> bool {
    move |e| matches!(e, Error::Forbidden { capability: c } if c == capability)
}
/// `Invalid` with a problem at `field` (and `code` when given).
pub(crate) fn invalid_at(
    field: &'static str,
    code: Option<&'static str>,
) -> impl Fn(&Error) -> bool {
    move |e| match e {
        Error::Invalid(ps) => ps
            .iter()
            .any(|p| p.field == field && code.is_none_or(|c| p.reason == c)),
        _ => false,
    }
}
