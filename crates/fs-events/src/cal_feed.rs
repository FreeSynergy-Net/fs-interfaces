//! Subscriptions and accounts: subscribe, fetch (`refresh_calendar`).
//!
//! Pattern: Facade over the state. The fake "fetches" from a table of feeds the test fills (`with_feed`); a feed
//! that is not in the table fails like an unreachable server. A failed fetch is a state, not an error.

use crate::cal_manage::refresh_ok;
use crate::draft::{
    Appointment, BasicValue, CalendarInfo, CalendarSource, Error, FetchState, HistoryKind,
    ImportReport, MediaType, ObjectId, Origin, OriginChannel, Reference, Refresh, Refreshed,
    Result, Revision, Subscription, Utc,
};
use crate::fake::FakeAppointments;
use crate::ids::calendar_iri;
use crate::sample::{at, timed};
use crate::scope::invalid;
use crate::state::{Item, State, blank};

fn forbidden(capability: &str) -> Error {
    Error::Forbidden {
        capability: capability.to_owned(),
    }
}

fn origin(channel: OriginChannel, cal: ObjectId, uid: &str, now: Utc) -> Origin {
    Origin {
        channel,
        source: calendar_iri(cal),
        media_type: MediaType("text/calendar".to_owned()),
        foreign_uid: uid.to_owned(),
        foreign_recurrence_id: None,
        foreign_etag: None,
        prod_id: None,
        imported: now,
        importer: "fs-calendars-testdata".to_owned(),
        rev_at_import: Revision(1),
        original: None,
    }
}

fn failed(old: &FetchState, now: Utc) -> FetchState {
    let (last_success, attempts) = match old {
        FetchState::Fresh { at } => (Some(*at), 1),
        FetchState::Failed {
            last_success,
            attempts,
            ..
        } => (*last_success, attempts + 1),
        _ => (None, 1),
    };
    FetchState::Failed {
        at: now,
        reason: "fetch.failed".to_owned(),
        last_success,
        attempts,
    }
}

impl FakeAppointments {
    pub(crate) fn subscribe_calendar_impl(
        &self,
        name: &str,
        color: Option<&str>,
        url: &Reference,
        refresh: Refresh,
    ) -> Result<CalendarInfo> {
        let mut s = self.lock();
        if s.asker_is_ai() {
            return Err(forbidden("calendar.subscribe"));
        }
        if name.trim().is_empty() {
            return Err(invalid("name", "must not be empty"));
        }
        if let Some(code) = crate::ssrf::refusal(url, &s.dns) {
            return Err(invalid("url", code));
        }
        refresh_ok(refresh)?;
        let info = CalendarInfo {
            id: s.fresh_id(),
            name: name.to_owned(),
            color: color.map(str::to_owned),
            default_zone: None,
            owner: s.viewer.clone(),
            source: CalendarSource::Subscribed(Subscription {
                url: Some(url.clone()),
                refresh,
                state: FetchState::Never,
            }),
            trash: None,
            revision: Revision(1),
        };
        s.calendars.push(info.clone());
        s.record_calendar(info.id, HistoryKind::Create, Vec::new());
        Ok(s.calendar_view(&info))
    }

    pub(crate) fn refresh_calendar_impl(&self, id: ObjectId) -> Result<Refreshed> {
        let mut s = self.lock();
        let cal = s.calendar(id)?.clone();
        if s.cal_deleted.contains_key(&id) {
            return Err(Error::NotFound(id));
        }
        let (address, old, channel) = match &cal.source {
            CalendarSource::Own => {
                return Err(invalid(
                    "calendar",
                    "is neither a subscription nor an account",
                ));
            }
            CalendarSource::Subscribed(sub) => (
                sub.url.clone(),
                sub.state.clone(),
                OriginChannel::Subscription,
            ),
            CalendarSource::Account { collection, state } => {
                (collection.clone(), state.clone(), OriginChannel::CalDav)
            }
        };
        if let Some(code) = address
            .as_ref()
            .and_then(|a| crate::ssrf::refusal(a, &s.dns))
        {
            return Err(invalid("url", code));
        }
        let now = s.now;
        let feed = address.and_then(|a| s.feeds.get(&a.canonical()).cloned());
        let mut report = ImportReport::default();
        let (mut updated, mut removed) = (Vec::new(), Vec::new());
        let state = if let Some(entries) = feed {
            sync(
                &mut s,
                id,
                channel,
                &entries,
                (&mut report, &mut updated, &mut removed),
            );
            FetchState::Fresh { at: now }
        } else {
            failed(&old, now)
        };
        let view = {
            let c = s.calendars.iter_mut().find(|c| c.id == id).map(|c| {
                match &mut c.source {
                    CalendarSource::Subscribed(sub) => sub.state = state.clone(),
                    CalendarSource::Account { state: st, .. } => *st = state.clone(),
                    CalendarSource::Own => {}
                }
                c.clone()
            });
            c.unwrap_or(cal)
        };
        s.record_calendar(id, HistoryKind::Update, Vec::new());
        Ok(Refreshed {
            calendar: s.calendar_view(&view),
            report,
            updated,
            removed,
        })
    }
}

/// Bring the calendar in line with what the source delivers: new ones come in, changed ones are updated, the ones
/// the source no longer has go to the trash (never for good).
fn sync(
    s: &mut State,
    cal: ObjectId,
    channel: OriginChannel,
    entries: &[(String, String)],
    out: (&mut ImportReport, &mut Vec<ObjectId>, &mut Vec<ObjectId>),
) {
    let (report, updated, removed) = out;
    let now = s.now;
    for (uid, title) in entries {
        let known = s
            .items
            .values()
            .find(|i| i.appt.calendar == cal && i.appt.uid == *uid && i.deleted.is_none())
            .map(|i| i.appt.clone());
        if let Some(mut a) = known {
            if a.title != *title {
                a.title.clone_from(title);
                a.revision = Revision(a.revision.0 + 1);
                a.updated = now;
                s.record(&a, HistoryKind::Update, Vec::new());
                updated.push(a.id);
                s.items.insert(
                    a.id,
                    Item {
                        appt: a,
                        deleted: None,
                        via_calendar: false,
                    },
                );
            }
            continue;
        }
        let id = s.fresh_id();
        let when = timed(at(2026, 12, 1, 10, 0), "Europe/Berlin", 60);
        let mut a: Appointment = blank(id, cal, uid.clone(), title, when, now);
        a.origin = Some(origin(channel, cal, uid, now));
        s.record(&a, HistoryKind::Create, Vec::new());
        s.items.insert(
            id,
            Item {
                appt: a,
                deleted: None,
                via_calendar: false,
            },
        );
        report.created.push(id);
    }
    let gone: Vec<Appointment> = s
        .items
        .values()
        .filter(|i| i.appt.calendar == cal && i.deleted.is_none())
        .filter(|i| !entries.iter().any(|(u, _)| *u == i.appt.uid))
        .map(|i| i.appt.clone())
        .collect();
    for a in gone {
        if let Some(i) = s.items.get_mut(&a.id) {
            i.deleted = Some(now);
        }
        s.record(&a, HistoryKind::Delete, Vec::new());
        removed.push(a.id);
    }
}
