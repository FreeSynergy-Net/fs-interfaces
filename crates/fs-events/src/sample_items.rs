//! The appointments of the sample, one function per case of the plan (section 5).
//!
//! Pattern: none (data). Dates around "now" (2026-11-16, a Monday); the two zone-change days are real.

use crate::draft::{
    Appointment, AppointmentPatch, Frequency, NDay, Override, OverrideChange, Privacy, Recurrence,
    RuleEnd, Utc, Weekday, When,
};
use crate::ids::object_id;
use crate::sample::{all_day, at, rule, timed};
use crate::state::blank;
use crate::zone;

fn now() -> Utc {
    Utc::from_millis(1_794_816_000_000)
}

pub(crate) struct Maker {
    pub(crate) n: u64,
    pub(crate) out: Vec<(&'static str, Appointment)>,
}

impl Maker {
    pub(crate) fn add(
        &mut self,
        slug: &'static str,
        calendar: u64,
        title: &str,
        when: When,
        tweak: impl FnOnce(&mut Appointment),
    ) {
        self.n += 1;
        let mut a = blank(
            object_id(1000 + self.n),
            object_id(calendar),
            format!("urn:fs-testdata:{slug}"),
            title,
            when,
            now(),
        );
        tweak(&mut a);
        self.out.push((slug, a));
    }
}

fn weekdays(days: &[Weekday]) -> Vec<NDay> {
    days.iter().map(|d| NDay { day: *d, nth: None }).collect()
}

/// All appointments of the sample (single cases first, then the 30 of one busy day).
#[must_use]
pub fn appointments() -> Vec<(&'static str, Appointment)> {
    let mut m = Maker {
        n: 0,
        out: Vec::new(),
    };
    singles(&mut m);
    series(&mut m);
    crate::sample_people::zone_cases(&mut m);
    crate::sample_people::social(&mut m);
    crate::sample_cals::calendar_cases(&mut m);
    for i in 1..=30_u32 {
        let slug: &'static str = Box::leak(format!("busy-day-{i:02}").into_boxed_str());
        let (hour, minute) = (
            7 + u8::try_from(i / 3).unwrap_or(0),
            u8::try_from(i % 3 * 20).unwrap_or(0),
        );
        m.add(
            slug,
            1,
            &format!("Termin {i}"),
            timed(at(2026, 11, 24, hour, minute), "Europe/Berlin", 20),
            |_| {},
        );
    }
    m.out
}

fn singles(m: &mut Maker) {
    m.add(
        "single-berlin",
        1,
        "Zahnarzt",
        timed(at(2026, 11, 17, 9, 0), "Europe/Berlin", 60),
        |_| {},
    );
    m.add(
        "single-new-york",
        2,
        "Call New York",
        timed(at(2026, 11, 17, 9, 0), "America/New_York", 60),
        |_| {},
    );
    m.add(
        "all-day-1",
        1,
        "Buß- und Bettag",
        all_day(2026, 11, 18, 1),
        |_| {},
    );
    m.add("all-day-3", 1, "Urlaub", all_day(2026, 12, 24, 3), |_| {});
    m.add(
        "private",
        1,
        "Privat",
        timed(at(2026, 11, 20, 20, 0), "Europe/Berlin", 90),
        |a| a.privacy = Privacy::Private,
    );
    m.add(
        "flight",
        1,
        "Flug FRA-JFK",
        When::Timed {
            start: at(2026, 11, 25, 10, 30),
            zone: zone::iana("Europe/Berlin"),
            length: crate::draft::Span {
                days: 0,
                seconds: 8 * 3600 + 30 * 60,
            },
            end_zone: Some(zone::iana("America/New_York")),
            zone_assumed: false,
        },
        |_| {},
    );
}

fn series(m: &mut Maker) {
    // Mondays and Wednesdays 10:00, 20 times from 2026-11-02; the 18th moved to 11:00, the 23rd cancelled.
    m.add(
        "weekly",
        2,
        "Teamrunde",
        timed(at(2026, 11, 2, 10, 0), "Europe/Berlin", 30),
        |a| {
            let mut r = rule(Frequency::Weekly);
            r.by_day = weekdays(&[Weekday::Mo, Weekday::We]);
            r.end = RuleEnd::Count(20);
            a.recurrence = vec![Recurrence::Rule(r)];
            a.overrides = vec![
                Override {
                    recurrence_id: at(2026, 11, 18, 10, 0),
                    change: OverrideChange::Changed(AppointmentPatch {
                        title: Some("Teamrunde (verlegt)".to_owned()),
                        when: Some(timed(at(2026, 11, 18, 11, 0), "Europe/Berlin", 30)),
                        ..AppointmentPatch::default()
                    }),
                },
                Override {
                    recurrence_id: at(2026, 11, 23, 10, 0),
                    change: OverrideChange::Excluded,
                },
            ];
        },
    );
    // Last working day of every month, 17:00.
    m.add(
        "monthly-last-workday",
        2,
        "Monatsabschluss",
        timed(at(2026, 1, 30, 17, 0), "Europe/Berlin", 60),
        |a| {
            let mut r = rule(Frequency::Monthly);
            r.by_day = weekdays(&[
                Weekday::Mo,
                Weekday::Tu,
                Weekday::We,
                Weekday::Th,
                Weekday::Fr,
            ]);
            r.by_set_position = vec![-1];
            a.recurrence = vec![Recurrence::Rule(r)];
        },
    );
}
