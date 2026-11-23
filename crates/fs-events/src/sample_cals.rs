//! The calendar cases of the sample: a subscription, a `CalDAV` account, a calendar in the trash, a series the fake
//! cannot compute (original rule).
//!
//! Pattern: none (data). The people are invented; the addresses are `example.org`.

use crate::draft::{
    CalendarInfo, CalendarSource, FetchState, MediaType, Origin, OriginChannel, Recurrence,
    Refresh, Restorable, Revision, Subscription, Utc,
};
use crate::ids::{calendar_iri, object_id, person};
use crate::sample::{all_day, at, timed};
use crate::sample_items::Maker;
use crate::zone::reference;

/// "Now" of the sample (2026-11-16T08:00:00Z).
const NOW: i64 = 1_794_816_000_000;

/// The ids of the added calendars: a subscription, an account, one in the trash.
pub const SUBSCRIBED: u64 = 3;
/// The `CalDAV` account calendar.
pub const ACCOUNT: u64 = 4;
/// The calendar in the trash (with one appointment).
pub const IN_TRASH: u64 = 5;

/// The address of the subscription (a secret in real life; the sample's is invented).
pub const FEED: &str = "https://kalender.example.org/feiertage-nord.ics";
/// The collection of the account.
pub const COLLECTION: &str = "https://dav.example.org/lena.ostrowski/privat/";

/// The three added calendars, owned by person 1.
#[must_use]
pub fn calendars() -> Vec<CalendarInfo> {
    let make =
        |n: u64, name: &str, source: CalendarSource, trash: Option<Restorable>| CalendarInfo {
            id: object_id(n),
            name: name.to_owned(),
            color: None,
            default_zone: None,
            owner: person(1),
            source,
            trash,
            revision: Revision(1),
        };
    let fresh = FetchState::Fresh {
        at: Utc::from_millis(NOW - 3_600_000),
    };
    vec![
        make(
            SUBSCRIBED,
            "Feiertage Nord (Abo)",
            CalendarSource::Subscribed(Subscription {
                url: Some(reference(FEED)),
                refresh: Refresh::Every { seconds: 86_400 },
                state: fresh.clone(),
            }),
            None,
        ),
        make(
            ACCOUNT,
            "Privat (CalDAV)",
            CalendarSource::Account {
                collection: Some(reference(COLLECTION)),
                state: fresh,
            },
            None,
        ),
        make(
            IN_TRASH,
            "Altes Projekt",
            CalendarSource::Own,
            Some(Restorable::Until(Utc::from_millis(NOW + 29 * 86_400_000))),
        ),
    ]
}

fn from(channel: OriginChannel, cal: u64, uid: &str) -> Origin {
    Origin {
        channel,
        source: calendar_iri(object_id(cal)),
        media_type: MediaType("text/calendar".to_owned()),
        foreign_uid: uid.to_owned(),
        foreign_recurrence_id: None,
        foreign_etag: Some("\"v1\"".to_owned()),
        prod_id: None,
        imported: Utc::from_millis(NOW - 3_600_000),
        importer: "fs-calendars-testdata".to_owned(),
        rev_at_import: Revision(1),
        original: None,
    }
}

pub(crate) fn calendar_cases(m: &mut Maker) {
    m.add(
        "feed-holiday",
        SUBSCRIBED,
        "Reformationstag",
        all_day(2027, 10, 31, 1),
        |a| {
            a.origin = Some(from(
                OriginChannel::Subscription,
                SUBSCRIBED,
                "feed-1@example.org",
            ));
        },
    );
    m.add(
        "feed-holiday-2",
        SUBSCRIBED,
        "Buß- und Bettag",
        all_day(2027, 11, 17, 1),
        |a| {
            a.origin = Some(from(
                OriginChannel::Subscription,
                SUBSCRIBED,
                "feed-2@example.org",
            ));
        },
    );
    m.add(
        "account-dentist",
        ACCOUNT,
        "Hausarzt (Kontrolle)",
        timed(at(2027, 2, 9, 9, 30), "Europe/Berlin", 45),
        |a| a.origin = Some(from(OriginChannel::CalDav, ACCOUNT, "dav-1@example.org")),
    );
    m.add(
        "trash-kickoff",
        IN_TRASH,
        "Projektstart (altes Projekt)",
        timed(at(2027, 2, 10, 10, 0), "Europe/Berlin", 60),
        |_| {},
    );
    // A rule outside the subset the fake computes: kept as text, shows its first occurrence only (ADR 0030).
    m.add(
        "original-series",
        1,
        "Maifeier-Planung (Regel nur als Text)",
        timed(at(2027, 5, 17, 18, 0), "Europe/Berlin", 90),
        |a| {
            a.recurrence = vec![Recurrence::Original(
                "FREQ=YEARLY;BYWEEKNO=20;BYDAY=MO".to_owned(),
            )];
        },
    );
}
