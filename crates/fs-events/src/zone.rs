//! From wall-clock time in a zone to an instant (ADR 0085 time cases).
//!
//! Pattern: Strategy (a zone rule is either an IANA zone or a fixed offset of a custom zone).
//! Gap (spring): the offset BEFORE the gap applies; ambiguity (autumn): the FIRST occurrence.

use std::collections::HashMap;

use chrono::{Duration, LocalResult, NaiveDate, NaiveDateTime, Offset, TimeZone, Timelike};
use chrono_tz::Tz;

use crate::draft::{
    BasicValue, Date, Error, LocalDateTime, Problem, Reference, Result, Span, TypeProblem, Utc,
    When, Zone, ZoneKind,
};

/// Custom zones (`/...`, from a `VTIMEZONE`): name to fixed offset in seconds east of UTC.
pub type CustomZones = HashMap<String, i32>;

/// A resolved zone.
#[derive(Clone, Copy, Debug)]
pub enum ZoneRule {
    /// An IANA zone.
    Iana(Tz),
    /// A custom zone, reduced to one fixed offset (the fake does not interpret `VTIMEZONE` rules).
    Fixed(i32),
}

/// Find the rule of a zone; an unknown one is `Invalid` at the field `zone`, never guessed.
///
/// # Errors
/// `Invalid` (code `zone.bad`) when the name is neither an IANA zone nor a registered custom zone.
pub fn resolve(zone: &Zone, customs: &CustomZones) -> Result<ZoneRule> {
    resolve_at(zone, customs, "zone")
}

/// [`resolve`] with the field the problem names (`when.zone`, `view_zone`, `default_zone` ...).
///
/// # Errors
/// `Invalid` (code `zone.bad`) at `field`; an unknown zone is not an error of its own (typ swap, decision 2a).
pub fn resolve_at(zone: &Zone, customs: &CustomZones, field: &str) -> Result<ZoneRule> {
    let name = zone.id();
    let rule = match zone.kind() {
        ZoneKind::Iana => name.parse::<Tz>().ok().map(ZoneRule::Iana),
        ZoneKind::Custom => customs.get(name).map(|o| ZoneRule::Fixed(*o)),
    };
    rule.ok_or_else(|| Error::Invalid(vec![Problem::from_type(field, TypeProblem::BadZone)]))
}

/// An IANA-shaped zone; the name need not exist (`resolve` says `UnknownZone`).
///
/// # Panics
/// On an empty name or one starting with `/` (a test-data mistake).
#[must_use]
pub fn iana(name: &str) -> Zone {
    Zone::iana(name).expect("a zone name of the test data")
}

/// A custom zone (`/...`).
///
/// # Panics
/// When the name does not start with `/` or is only that (a test-data mistake).
#[must_use]
pub fn custom(name: &str) -> Zone {
    Zone::custom(name).expect("a custom zone name of the test data")
}

/// An absolute reference (`urn:...`, `https://...`).
///
/// # Panics
/// When `text` is not absolute (a test-data mistake).
#[must_use]
pub fn reference(text: &str) -> Reference {
    Reference::parse(text).expect("a reference of the test data")
}

impl ZoneRule {
    /// The wall-clock time as a naive UTC time.
    #[must_use]
    pub fn to_utc(self, local: NaiveDateTime) -> NaiveDateTime {
        let offset = match self {
            ZoneRule::Fixed(seconds) => seconds,
            ZoneRule::Iana(tz) => match tz.offset_from_local_datetime(&local) {
                LocalResult::Single(o) | LocalResult::Ambiguous(o, _) => o.fix().local_minus_utc(),
                LocalResult::None => {
                    match tz.offset_from_local_datetime(&(local - Duration::hours(24))) {
                        LocalResult::Single(o) | LocalResult::Ambiguous(o, _) => {
                            o.fix().local_minus_utc()
                        }
                        LocalResult::None => 0,
                    }
                }
            },
        };
        local - Duration::seconds(i64::from(offset))
    }
}

/// `Date` to a calendar date, `None` if it does not exist.
#[must_use]
pub fn naive_date(d: Date) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(&d.canonical(), "%Y-%m-%d").ok()
}

/// A calendar date as `Date`.
#[must_use]
pub fn plain(d: NaiveDate) -> Date {
    Date::parse(&d.format("%Y-%m-%d").to_string()).unwrap_or_else(|_| epoch_date())
}

fn epoch_date() -> Date {
    Date::parse("1970-01-01").unwrap_or_else(|_| unreachable!("a fixed valid date"))
}

/// `LocalDateTime` to a naive date-time, `None` if it does not exist.
#[must_use]
pub fn naive(l: LocalDateTime) -> Option<NaiveDateTime> {
    naive_date(l.date())?.and_hms_opt(
        u32::from(l.hour()),
        u32::from(l.minute()),
        u32::from(l.second()),
    )
}

/// A naive date-time as `LocalDateTime`.
#[must_use]
pub fn local(n: NaiveDateTime) -> LocalDateTime {
    LocalDateTime::new(
        plain(n.date()),
        u8::try_from(n.hour()).unwrap_or(0),
        u8::try_from(n.minute()).unwrap_or(0),
        u8::try_from(n.second()).unwrap_or(0),
    )
    .unwrap_or_else(|_| unreachable!("hour, minute and second of a chrono time are in range"))
}

/// Milliseconds since the epoch of a naive UTC time.
#[must_use]
pub fn to_utc_ms(n: NaiveDateTime) -> Utc {
    Utc::from_millis(n.and_utc().timestamp_millis())
}

/// A naive UTC time from milliseconds.
#[must_use]
pub fn from_utc_ms(u: Utc) -> NaiveDateTime {
    chrono::DateTime::from_timestamp_millis(u.millis())
        .map_or(chrono::DateTime::UNIX_EPOCH.naive_utc(), |d| d.naive_utc())
}

fn invalid(field: &str, reason: &str) -> Error {
    Error::Invalid(vec![Problem {
        field: field.to_owned(),
        reason: reason.to_owned(),
    }])
}

/// The first and the last instant of one occurrence. A timed one: start in its zone, end = start plus the nominal
/// days (in the zone) plus the seconds (exact). An all-day one and a floating one have no zone of their own:
/// they are placed in the zone of the viewer `view` (ADR 0085), so their instants differ per viewer.
///
/// # Errors
/// `Invalid` for an unknown zone or for a date or time that does not exist.
pub fn bounds(when: &When, customs: &CustomZones, view: &Zone) -> Result<(Utc, Utc)> {
    match when {
        When::Timed {
            start,
            zone,
            length,
            ..
        } => placed(resolve_at(zone, customs, "when.zone")?, *start, *length),
        When::Floating { start, length } => {
            placed(resolve_at(view, customs, "view_zone")?, *start, *length)
        }
        When::AllDay { first, days } => {
            let rule = resolve_at(view, customs, "view_zone")?;
            let d = naive_date(*first).ok_or_else(|| invalid("when.first", "no such date"))?;
            let s = d.and_time(chrono::NaiveTime::MIN);
            Ok((
                to_utc_ms(rule.to_utc(s)),
                to_utc_ms(rule.to_utc(s + Duration::days(i64::from(*days)))),
            ))
        }
    }
}

fn placed(rule: ZoneRule, start: LocalDateTime, length: Span) -> Result<(Utc, Utc)> {
    let n = naive(start).ok_or_else(|| invalid("when.start", "no such date or time"))?;
    Ok((to_utc_ms(rule.to_utc(n)), end_of(rule, n, length)))
}

fn end_of(rule: ZoneRule, start: NaiveDateTime, length: Span) -> Utc {
    let end_local = start + Duration::days(i64::from(length.days));
    Utc::from_millis(to_utc_ms(rule.to_utc(end_local)).millis() + i64::from(length.seconds) * 1000)
}
