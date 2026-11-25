//! The sample as test data for pictures (F5): calendar colours, people that are the people of the directory sample,
//! and the synchronous expansion of a series in a window.
//!
//! Pattern: none (tests). The colour and people tests go red when the data change they protect is taken back; the series test documents
//! behaviour that already existed (it would stay green without this branch).

mod common;

use std::collections::{HashMap, HashSet};

use common::{utc, utc_zone};
use fs_events::draft::PersonArt;
use fs_events::instances::in_window;
use fs_events::sample::{CUSTOM_OFFSET, CUSTOM_ZONE};
use fs_events::{Sample, sample};
use fs_person_directory::{PersonDirectory, Resolved, sample as directory_sample, sample_name};

fn rgb(text: &str) -> Option<[u8; 3]> {
    let hex = text.strip_prefix('#')?;
    (hex.len() == 6 && hex.is_ascii()).then_some(())?;
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

#[test]
fn every_calendar_has_its_own_fixed_colour() {
    let s: Sample = sample();
    assert_eq!(s.calendars.len(), 5);
    let mut seen = HashSet::new();
    for c in &s.calendars {
        let colour = c.color.as_deref().unwrap_or_else(|| {
            panic!("calendar `{}` has no colour (RFC 8984 `color`)", c.name);
        });
        assert!(
            rgb(colour).is_some(),
            "`{colour}` of `{}` is not #rrggbb",
            c.name
        );
        assert!(seen.insert(colour.to_owned()), "`{colour}` is used twice");
    }
    // The same on every run (no randomness): the named constants are what the calendars carry.
    let by_name = |n: &str| {
        s.calendars
            .iter()
            .find(|c| c.name == n)
            .and_then(|c| c.color.clone())
    };
    assert_eq!(
        by_name("Persönlich").as_deref(),
        Some(fs_events::sample::COLOR_PERSONAL)
    );
    assert_eq!(
        by_name("Arbeit").as_deref(),
        Some(fs_events::sample::COLOR_WORK)
    );
}

#[tokio::test]
async fn sample_people_are_the_people_of_the_directory_sample() {
    let events = sample();
    let directory = directory_sample();
    for known in &events.people {
        let resolved = directory.resolve(&known.person).await.expect("resolve");
        let Resolved::Found(view) = resolved else {
            panic!("`{}` does not resolve in the directory sample", known.name);
        };
        assert_eq!(view.person.art, PersonArt::Person, "{}", known.name);
        assert_eq!(
            view.display, known.name,
            "the name comes from the directory sample"
        );
    }
    // The acting person of the events sample is the asker of the directory sample.
    let asker = directory.asker().await.expect("asker").expect("an asker");
    assert_eq!(asker.id, events.people[0].person.id);
    assert_eq!(events.people[0].name, sample_name(1).unwrap());
}

#[test]
fn a_series_is_expanded_synchronously_in_a_window() {
    let s = sample();
    let customs: HashMap<String, i32> = HashMap::from([(CUSTOM_ZONE.to_owned(), CUSTOM_OFFSET)]);
    let (found, damaged) = in_window(
        s.get("weekly"),
        &customs,
        None,
        &utc_zone(),
        utc(2026, 11, 16, 0, 0),
        utc(2026, 11, 24, 0, 0),
    );
    assert_eq!(damaged.len(), 0);
    // Mo 16th 10:00, We 18th moved to 11:00 Berlin, Mo 23rd cancelled.
    let mut starts: Vec<_> = found.iter().map(|i| i.start).collect();
    starts.sort();
    assert_eq!(
        starts,
        vec![utc(2026, 11, 16, 9, 0), utc(2026, 11, 18, 10, 0)]
    );
}

#[test]
fn the_shared_people_have_the_address_of_the_directory_sample() {
    let events = sample();
    for (i, entry) in [1_u8, 10, 12, 4].into_iter().enumerate() {
        assert_eq!(
            Some(events.people[i].address.as_str()),
            fs_person_directory::sample_address(entry),
            "person {} is entry {entry}",
            i + 1
        );
    }
}
