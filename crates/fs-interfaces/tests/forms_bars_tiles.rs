//! The two shared forms `bars-in-time@1.0.0` (calendar, task Gantt, workflow Gantt) and `tiles@1.0.0` (screenshots, file grid): the channels, their art, the
//! required ones and the count are pinned, so a missing channel such as `label` or a wrong version notation turns the test red. The standards of the
//! channels (RFC 3339 for times, RFC 9253 for `depends`) stand in the comment lines of `schema/forms.txt` and are pinned as text, so they cannot silently go.

use fs_interfaces::forms::{self, Art, Count, parse};

fn names<'a>(it: impl Iterator<Item = &'a forms::Channel>) -> Vec<&'a str> {
    it.map(|c| c.name.as_str()).collect()
}

#[test]
fn bars_in_time_has_the_four_required_channels_and_the_shared_optional_ones() {
    let f = forms::find_key("bars-in-time@1.0.0").unwrap();
    assert_eq!(names(f.required()), ["items", "start", "end", "label"]);
    for (name, art) in [
        ("items", Art::Records),
        ("start", Art::Time),
        ("end", Art::Time),
        ("label", Art::Text),
        ("lane", Art::Text),
        ("state", Art::Text),
        ("depends", Art::Reference),
        ("progress", Art::Number),
        ("all-day", Art::Flag),
        ("move", Art::Command),
    ] {
        let c = f.channel(name).unwrap_or_else(|| panic!("channel {name}"));
        assert_eq!(c.arts, [art], "{name}");
    }
    assert_eq!(f.channels.len(), 10);
    assert_eq!(
        f.channel("depends").unwrap().count,
        Count { min: 0, max: None }
    );
}

#[test]
fn tiles_has_only_items_and_label_required() {
    let f = forms::find_key("tiles@1.0.0").unwrap();
    assert_eq!(names(f.required()), ["items", "label"]);
    assert_eq!(f.channel("preview").unwrap().arts, [Art::Reference]);
    assert_eq!(f.channel("time").unwrap().arts, [Art::Time]);
    for (name, art) in [
        ("items", Art::Records),
        ("label", Art::Text),
        ("preview", Art::Reference),
        ("kind", Art::Text),
        ("time", Art::Time),
        ("size", Art::Number),
        ("order", Art::Number),
        ("open", Art::Command),
    ] {
        let c = f.channel(name).unwrap_or_else(|| panic!("channel {name}"));
        assert_eq!(c.arts, [art], "{name}");
    }
    assert_eq!(
        f.channel("facts").unwrap().count,
        Count {
            min: 0,
            max: Some(4)
        }
    );
    assert_eq!(f.channels.len(), 9);
}

#[test]
fn the_two_part_and_one_part_notation_find_nothing() {
    for key in ["bars-in-time@1.0", "bars-in-time@1", "tiles@1.0", "tiles@1"] {
        assert!(forms::find_key(key).is_none(), "{key}");
    }
}

/// The standards of the channels are named in the catalog (24.11.2026): times RFC 3339, `depends` RFC 5545 RELATED-TO with RFC 9253.
#[test]
fn the_standards_of_the_time_and_depends_channels_are_named() {
    let s = forms::SOURCE;
    for needle in [
        "RFC 3339",
        "RFC 9253",
        "DEPENDS-ON",
        "FINISHTOSTART",
        "RELATED-TO",
    ] {
        assert!(s.contains(needle), "{needle}");
    }
}

/// Mutants: a form without `label`, or with the required channel set to optional, is a different form; the pin above would see it.
#[test]
fn a_bars_form_without_label_has_a_different_required_set() {
    // Only the label line of bars-in-time@1.0.0 goes (1.1.0 and checklist name `label` in a `writes` and would stop parsing).
    let mut in_bars = false;
    let text = forms::SOURCE
        .lines()
        .filter(|l| {
            if !l.starts_with('>') && !l.starts_with('#') && !l.is_empty() {
                in_bars = l.starts_with("bars-in-time@1.0.0");
            }
            !(in_bars && l.starts_with("> label | text | ! | 1"))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let all = parse(&text).unwrap();
    let f = all
        .iter()
        .find(|f| f.key() == "bars-in-time@1.0.0")
        .unwrap();
    assert_ne!(names(f.required()), ["items", "start", "end", "label"]);
}
