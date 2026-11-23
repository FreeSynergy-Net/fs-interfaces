//! The zone cases and the people cases of the sample (split off `sample_items.rs`).
//!
//! Pattern: none (data).

use crate::draft::{
    Alert, AlertAction, Attendee, Location, MediaType, Origin, OriginChannel, Participant,
    ParticipationStatus, RelativeTo, Revision, Roles, Trigger, Utc, When,
};
use crate::ids::person;
use crate::sample::{CUSTOM_ZONE, at, timed};
use crate::sample_items::Maker;
use crate::zone;

fn now() -> Utc {
    Utc::from_millis(1_794_816_000_000)
}

fn who(key: &str, n: u64, status: ParticipationStatus, owner: bool) -> Participant {
    Participant {
        key: key.to_owned(),
        who: Attendee::Actor(person(n)),
        roles: Roles {
            owner,
            attendee: true,
            ..Roles::default()
        },
        status,
        expect_reply: !owner,
        comment: None,
        send_to: Vec::new(),
        invited_by: None,
        delegated_to: Vec::new(),
        delegated_from: Vec::new(),
        schedule_sequence: 0,
        schedule_updated: None,
    }
}

/// Where the imported sample came from: a file of `text/calendar`, unchanged since the import.
fn imported_file() -> Origin {
    Origin {
        channel: OriginChannel::File,
        source: zone::reference(
            "urn:sha256:0000000000000000000000000000000000000000000000000000000000000001",
        ),
        media_type: MediaType("text/calendar".to_owned()),
        foreign_uid: "floating-imported@example.org".to_owned(),
        foreign_recurrence_id: None,
        foreign_etag: None,
        prod_id: Some("-//Example//Calendar//EN".to_owned()),
        imported: now(),
        importer: "fs-calendars-ics 0.0.0".to_owned(),
        rev_at_import: Revision(1),
        original: None,
    }
}

fn room(key: &str) -> Participant {
    let mut p = who(key, 1, ParticipationStatus::Accepted, false);
    p.who = Attendee::Location(zone::reference(
        "urn:uuid:00000000-0000-7000-a000-000000000004",
    ));
    p.roles = Roles::default();
    p.expect_reply = false;
    p
}

pub(crate) fn zone_cases(m: &mut Maker) {
    m.add(
        "dst-gap",
        1,
        "Nachtwache (Lücke)",
        timed(at(2026, 3, 29, 2, 30), "Europe/Berlin", 60),
        |_| {},
    );
    m.add(
        "dst-double",
        1,
        "Nachtwache (Doppelung)",
        timed(at(2026, 10, 25, 2, 30), "Europe/Berlin", 60),
        |_| {},
    );
    m.add(
        "floating-imported",
        1,
        "Importiert (schwebend)",
        When::Timed {
            start: at(2026, 11, 19, 8, 0),
            zone: zone::iana("Europe/Berlin"),
            length: crate::draft::Span {
                days: 0,
                seconds: 3600,
            },
            end_zone: None,
            zone_assumed: true,
        },
        |a| a.origin = Some(imported_file()),
    );
    m.add(
        "floating",
        1,
        "Wecker (überall 8 Uhr)",
        When::Floating {
            start: at(2026, 11, 20, 8, 0),
            length: crate::draft::Span {
                days: 0,
                seconds: 1800,
            },
        },
        |_| {},
    );
    m.add(
        "custom-zone",
        2,
        "Außenstelle",
        When::Timed {
            start: at(2026, 11, 20, 14, 0),
            zone: zone::custom(CUSTOM_ZONE),
            length: crate::draft::Span {
                days: 0,
                seconds: 3600,
            },
            end_zone: None,
            zone_assumed: false,
        },
        |_| {},
    );
}

pub(crate) fn social(m: &mut Maker) {
    m.add(
        "invitation",
        2,
        "Planung",
        timed(at(2026, 11, 19, 15, 0), "Europe/Berlin", 60),
        |a| {
            a.participants = vec![
                who("p1", 1, ParticipationStatus::Accepted, true),
                who("p2", 2, ParticipationStatus::Accepted, false),
                who("p3", 3, ParticipationStatus::Declined, false),
                who("p4", 4, ParticipationStatus::NeedsAction, false),
                room("p5"),
            ];
            a.locations = vec![Location {
                key: "l1".to_owned(),
                name: "Raum 4".to_owned(),
                description: None,
                types: Vec::new(),
                coordinates: None,
                zone: None,
                relative_to: None,
                place: None,
            }];
        },
    );
    m.add(
        "two-alerts",
        1,
        "Arzttermin",
        timed(at(2026, 11, 21, 11, 0), "Europe/Berlin", 45),
        |a| {
            let alert = |key: &str, seconds: i64| Alert {
                key: key.to_owned(),
                trigger: Trigger::Offset {
                    seconds,
                    relative_to: RelativeTo::Start,
                },
                action: AlertAction::Display,
                acknowledged: None,
            };
            a.alerts = vec![alert("a1", -900), alert("a2", -86_400)];
        },
    );
}
