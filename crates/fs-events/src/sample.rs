//! The fixed sample: the appointments, calendars and people every frontend picture and every provider test uses.
//!
//! Pattern: Builder of fixed data (no randomness, no clock): the same ids, uids and times on every run.
//! The slug of an appointment is its name in tests; its `uid` is `urn:fs-testdata:<slug>`.

use crate::draft::{
    Appointment, BasicValue, CalendarInfo, CalendarSource, Date, Frequency, LocalDateTime,
    RecurrenceRule, Revision, RuleEnd, Skip, Span, Weekday, When,
};
use crate::fake::FakeAppointments;
use crate::ids::{object_id, person};
use crate::people::{FakePeople, Known};
use crate::sample_items::appointments;
use crate::zone;

/// The colours of the five sample calendars (RFC 8984 `color`, `#rrggbb`): fixed, named, easy to tell apart.
/// "Persönlich" blue.
pub const COLOR_PERSONAL: &str = "#1c71d8";
/// "Arbeit" orange.
pub const COLOR_WORK: &str = "#e66100";
/// The subscription (holidays) green.
pub const COLOR_HOLIDAYS: &str = "#26a269";
/// The `CalDAV` account purple.
pub const COLOR_ACCOUNT: &str = "#9141ac";
/// The calendar in the trash grey.
pub const COLOR_TRASH: &str = "#77767b";

/// The custom zone of the sample ("/Custom/Aussenstelle"), a fixed offset of +05:30.
pub const CUSTOM_ZONE: &str = "/Custom/Aussenstelle";
/// Its offset in seconds east of UTC.
pub const CUSTOM_OFFSET: i32 = 5 * 3600 + 30 * 60;

/// The whole sample.
#[derive(Clone, Debug)]
pub struct Sample {
    /// Five calendars: "Persönlich" (1), "Arbeit" (2), a subscription (3), a `CalDAV` account (4), one in the trash (5).
    pub calendars: Vec<CalendarInfo>,
    /// Four known people, by the references of the directory sample (`fs-person-directory`): the acting person Amara Zolenko (1 = entry 1), Soo-jin Marchetti (2 = entry 10), Tariq Lindqvist (3 = entry 12), Assistent Sol (4 = entry 4, an AI account).
    pub people: Vec<Known>,
    /// The appointments with their slugs, in a fixed order.
    pub appointments: Vec<(&'static str, Appointment)>,
}

impl Sample {
    /// The appointment with this slug.
    ///
    /// # Panics
    /// If the slug is not in the sample (a typo in a test).
    #[must_use]
    pub fn get(&self, slug: &str) -> &Appointment {
        self.appointments
            .iter()
            .find(|(s, _)| *s == slug)
            .map_or_else(|| panic!("no sample appointment `{slug}`"), |(_, a)| a)
    }

    /// A [`FakeAppointments`] loaded with the sample (the acting person is person 1).
    #[must_use]
    pub fn fake(&self) -> FakeAppointments {
        let fake = FakeAppointments::new()
            .with_custom_zone(CUSTOM_ZONE, CUSTOM_OFFSET)
            .with_viewer(person(1));
        fake.set_calendars(self.calendars.clone());
        for (_, a) in &self.appointments {
            fake.insert_raw(a.clone());
        }
        fake
    }

    /// A [`FakePeople`] with the sample's people.
    #[must_use]
    pub fn directory(&self) -> FakePeople {
        FakePeople::new(self.people.clone())
    }
}

/// The fixed sample.
#[must_use]
pub fn sample() -> Sample {
    let owner = person(1);
    let cal = |n: u64, name: &str, color: &str| CalendarInfo {
        id: object_id(n),
        name: name.to_owned(),
        color: Some(color.to_owned()),
        default_zone: None,
        owner: owner.clone(),
        source: CalendarSource::Own,
        trash: None,
        revision: Revision(1),
    };
    let mut calendars = vec![
        cal(1, "Persönlich", COLOR_PERSONAL),
        cal(2, "Arbeit", COLOR_WORK),
    ];
    calendars.extend(crate::sample_cals::calendars());
    Sample {
        calendars,
        people: vec![
            directory_known(1, "mailto:amara@example.org"),
            directory_known(2, "mailto:soojin.marchetti@example.org"),
            directory_known(3, "mailto:tariq.lindqvist@example.org"),
            directory_known(4, "mailto:assistent.sol@example.org"),
        ],
        appointments: appointments(),
    }
}

/// A known person of the sample: the IRI and the NAME are those of the directory sample (`fs-person-directory`), only
/// the address is chosen here.
fn directory_known(n: u64, address: &str) -> Known {
    let name = crate::ids::directory_entry(n)
        .and_then(fs_person_directory::sample_name)
        .unwrap_or("unknown sample person");
    FakePeople::known(n, name, address)
}

/// A wall-clock time (helper for the sample and for tests).
///
/// # Panics
/// On a date or time that does not exist (a test-data mistake).
#[must_use]
pub fn at(year: i32, month: u8, day: u8, hour: u8, minute: u8) -> LocalDateTime {
    LocalDateTime::new(date(year, month, day), hour, minute, 0).expect("a time of the test data")
}

/// A calendar date (helper for the sample and for tests).
///
/// # Panics
/// On a date that does not exist (a test-data mistake).
#[must_use]
pub fn date(year: i32, month: u8, day: u8) -> Date {
    Date::parse(&format!("{year:04}-{month:02}-{day:02}")).expect("a date of the test data")
}

/// A timed `When` in an IANA zone, `minutes` long.
#[must_use]
pub fn timed(start: LocalDateTime, zone: &str, minutes: u32) -> When {
    When::Timed {
        start,
        zone: zone::iana(zone),
        length: Span {
            days: 0,
            seconds: minutes * 60,
        },
        end_zone: None,
        zone_assumed: false,
    }
}

/// An all-day `When`.
///
/// # Panics
/// On a date that does not exist (a test-data mistake).
#[must_use]
pub fn all_day(year: i32, month: u8, day: u8, days: u32) -> When {
    When::AllDay {
        first: date(year, month, day),
        days,
    }
}

/// A rule with only the frequency set (interval 1, Monday week start, never ends).
#[must_use]
pub fn rule(frequency: Frequency) -> RecurrenceRule {
    RecurrenceRule {
        frequency,
        interval: 1,
        rscale: None,
        skip: Skip::Omit,
        first_day_of_week: Weekday::Mo,
        by_day: Vec::new(),
        by_month_day: Vec::new(),
        by_month: Vec::new(),
        by_year_day: Vec::new(),
        by_week_no: Vec::new(),
        by_hour: Vec::new(),
        by_minute: Vec::new(),
        by_second: Vec::new(),
        by_set_position: Vec::new(),
        end: RuleEnd::Never,
    }
}
