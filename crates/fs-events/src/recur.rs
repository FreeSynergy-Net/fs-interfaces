//! Expanding a recurrence rule in the zone of the appointment (wall-clock time, never UTC).
//!
//! Pattern: Iterator-like generator over periods. Supports daily, weekly, monthly and yearly rules with
//! interval, `by_day` (with `nth`), `by_month_day`, `by_month`, `by_set_position`, count and until. Everything
//! else is refused by [`check`] so a test never silently gets a wrong series.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Weekday as Wd};

use crate::draft::{Frequency, NDay, Problem, RecurrenceRule, RuleEnd, Skip, Weekday};
use crate::zone::naive;

const MAX_PERIODS: i64 = 40_000;

/// Reasons why the fake cannot expand this rule (empty = fine).
#[must_use]
pub fn check(rule: &RecurrenceRule) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut no = |field: &str, reason: &str| {
        out.push(Problem {
            field: format!("recurrence.{field}"),
            reason: reason.to_owned(),
        });
    };
    if matches!(
        rule.frequency,
        Frequency::Hourly | Frequency::Minutely | Frequency::Secondly
    ) {
        no(
            "frequency",
            "the fake expands daily, weekly, monthly and yearly rules only",
        );
    }
    if rule.interval == 0 {
        no("interval", "must be at least 1");
    }
    if rule
        .rscale
        .as_deref()
        .is_some_and(|r| !r.eq_ignore_ascii_case("gregorian"))
    {
        no("rscale", "only the gregorian calendar");
    }
    if !(rule.by_year_day.is_empty()
        && rule.by_week_no.is_empty()
        && rule.by_hour.is_empty()
        && rule.by_minute.is_empty()
        && rule.by_second.is_empty())
    {
        no(
            "by",
            "by_year_day, by_week_no, by_hour, by_minute, by_second are not expanded",
        );
    }
    if rule.by_month.iter().any(|m| m.leap) {
        no("by_month", "leap months need another calendar");
    }
    if rule.skip != Skip::Omit {
        no("skip", "only `omit`");
    }
    out
}

fn chrono_day(d: Weekday) -> Wd {
    match d {
        Weekday::Mo => Wd::Mon,
        Weekday::Tu => Wd::Tue,
        Weekday::We => Wd::Wed,
        Weekday::Th => Wd::Thu,
        Weekday::Fr => Wd::Fri,
        Weekday::Sa => Wd::Sat,
        Weekday::Su => Wd::Sun,
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let (ny, nm) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(ny, nm, 1)
        .and_then(|d| d.pred_opt())
        .map_or(28, |d| d.day())
}

fn nth_matches(year: i32, month: u32, rule_days: &[NDay]) -> Vec<NaiveDate> {
    let total = days_in_month(year, month);
    let mut out = Vec::new();
    for rd in rule_days {
        let all: Vec<u32> = (1..=total)
            .filter(|d| {
                NaiveDate::from_ymd_opt(year, month, *d)
                    .is_some_and(|x| x.weekday() == chrono_day(rd.day))
            })
            .collect();
        let picked: Vec<u32> = match rd.nth {
            None => all,
            Some(n) if n > 0 => all
                .get(usize::try_from(n - 1).unwrap_or(usize::MAX))
                .copied()
                .into_iter()
                .collect(),
            Some(n) => {
                let back = usize::try_from(-i32::from(n)).unwrap_or(usize::MAX);
                all.len()
                    .checked_sub(back)
                    .and_then(|i| all.get(i))
                    .copied()
                    .into_iter()
                    .collect()
            }
        };
        out.extend(
            picked
                .into_iter()
                .filter_map(|d| NaiveDate::from_ymd_opt(year, month, d)),
        );
    }
    out
}

fn month_dates(year: i32, month: u32, rule: &RecurrenceRule, default_day: u32) -> Vec<NaiveDate> {
    let mut dates = if !rule.by_day.is_empty() {
        nth_matches(year, month, &rule.by_day)
    } else if !rule.by_month_day.is_empty() {
        let total = i32::try_from(days_in_month(year, month)).unwrap_or(28);
        rule.by_month_day
            .iter()
            .filter_map(|d| {
                let day = if *d < 0 {
                    total + i32::from(*d) + 1
                } else {
                    i32::from(*d)
                };
                u32::try_from(day)
                    .ok()
                    .and_then(|day| NaiveDate::from_ymd_opt(year, month, day))
            })
            .collect()
    } else {
        NaiveDate::from_ymd_opt(year, month, default_day)
            .into_iter()
            .collect()
    };
    dates.sort();
    dates.dedup();
    if rule.by_set_position.is_empty() {
        return dates;
    }
    let len = i64::try_from(dates.len()).unwrap_or(0);
    let mut picked: Vec<NaiveDate> = rule
        .by_set_position
        .iter()
        .filter_map(|p| {
            let i = if *p > 0 {
                i64::from(*p) - 1
            } else {
                len + i64::from(*p)
            };
            usize::try_from(i).ok().and_then(|i| dates.get(i).copied())
        })
        .collect();
    picked.sort();
    picked.dedup();
    picked
}

fn add_months(date: NaiveDate, n: i64) -> (i32, u32) {
    let index = i64::from(date.year()) * 12 + i64::from(date.month0()) + n;
    (
        i32::try_from(index.div_euclid(12)).unwrap_or(1970),
        u32::try_from(index.rem_euclid(12)).unwrap_or(0) + 1,
    )
}

fn period(rule: &RecurrenceRule, start: NaiveDateTime, n: i64) -> Vec<NaiveDate> {
    let first = start.date();
    let filter_days = |d: &NaiveDate| {
        rule.by_day.is_empty() || rule.by_day.iter().any(|x| chrono_day(x.day) == d.weekday())
    };
    match rule.frequency {
        Frequency::Daily => {
            let d = first + Duration::days(n);
            let month_ok = rule.by_month.is_empty()
                || rule
                    .by_month
                    .iter()
                    .any(|m| u32::from(m.month) == d.month());
            if filter_days(&d) && month_ok {
                vec![d]
            } else {
                Vec::new()
            }
        }
        Frequency::Weekly => {
            let wkst = chrono_day(rule.first_day_of_week);
            let back = i64::from(
                (first.weekday().num_days_from_monday() + 7 - wkst.num_days_from_monday()) % 7,
            );
            let week_start = first - Duration::days(back) + Duration::weeks(n);
            let days: Vec<Weekday> = if rule.by_day.is_empty() {
                vec![weekday_of(first)]
            } else {
                rule.by_day.iter().map(|d| d.day).collect()
            };
            let mut out: Vec<NaiveDate> = days
                .into_iter()
                .map(|d| {
                    week_start
                        + Duration::days(i64::from(
                            (chrono_day(d).num_days_from_monday() + 7
                                - wkst.num_days_from_monday())
                                % 7,
                        ))
                })
                .collect();
            out.sort();
            out
        }
        Frequency::Monthly => {
            let (y, m) = add_months(first, n);
            month_dates(y, m, rule, first.day())
        }
        _ => {
            let year = first.year() + i32::try_from(n).unwrap_or(0);
            let months: Vec<u32> = if rule.by_month.is_empty() {
                vec![first.month()]
            } else {
                rule.by_month.iter().map(|m| u32::from(m.month)).collect()
            };
            let mut out: Vec<NaiveDate> = months
                .into_iter()
                .flat_map(|m| month_dates(year, m, rule, first.day()))
                .collect();
            out.sort();
            out
        }
    }
}

fn weekday_of(d: NaiveDate) -> Weekday {
    [
        Weekday::Mo,
        Weekday::Tu,
        Weekday::We,
        Weekday::Th,
        Weekday::Fr,
        Weekday::Sa,
        Weekday::Su,
    ][d.weekday().num_days_from_monday() as usize]
}

/// The occurrences of one rule, from `start` (always the first) up to `limit` (inclusive) and the end of the rule.
#[must_use]
pub fn occurrences(
    rule: &RecurrenceRule,
    start: NaiveDateTime,
    limit: NaiveDateTime,
) -> Vec<NaiveDateTime> {
    let until = match rule.end {
        RuleEnd::Until(l) => naive(l),
        _ => None,
    };
    let count = match rule.end {
        RuleEnd::Count(c) => Some(usize::try_from(c).unwrap_or(usize::MAX)),
        _ => None,
    };
    let step = i64::from(rule.interval.max(1));
    let time = start.time();
    let mut out: Vec<NaiveDateTime> = Vec::new();
    for k in 0..MAX_PERIODS {
        let mut dates = period(rule, start, k * step);
        if k == 0 && !dates.contains(&start.date()) {
            dates.push(start.date());
            dates.sort();
        }
        for d in dates {
            let at = d.and_time(time);
            if at < start {
                continue;
            }
            if at > limit || until.is_some_and(|u| at > u) || count.is_some_and(|c| out.len() >= c)
            {
                return out;
            }
            out.push(at);
        }
    }
    out
}
