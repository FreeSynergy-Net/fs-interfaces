//! CIC-B: `cards-in-columns@1.4.0` is appended: three optional channels (state, assignee, time-zone); 1.0.0 to 1.3.0 stay as they were.
//! Pattern: Projection – the catalog names the channels in standard values; grouping and labels belong to the frontend.

use fs_interfaces::forms::{self, Art};

const OLDER: [&str; 4] = [
    "cards-in-columns@1.0.0",
    "cards-in-columns@1.1.0",
    "cards-in-columns@1.2.0",
    "cards-in-columns@1.3.0",
];

#[test]
fn version_1_4_0_adds_three_optional_channels_and_keeps_1_3_0() {
    let old = forms::find_key("cards-in-columns@1.3.0").unwrap();
    let new = forms::find_key("cards-in-columns@1.4.0").unwrap();
    for c in &old.channels {
        let n = new.channel(&c.name).unwrap();
        assert_eq!(
            (&c.arts, c.required, c.count, &c.writes),
            (&n.arts, n.required, n.count, &n.writes),
            "{}",
            c.name
        );
    }
    let added = [
        ("state", Art::Text),
        ("assignee", Art::Reference),
        ("time-zone", Art::Text),
    ];
    assert_eq!(new.channels.len(), old.channels.len() + added.len());
    for (name, art) in added {
        let c = new.channel(name).unwrap();
        assert_eq!(c.arts, [art], "{name}");
        assert!(!c.required, "{name}");
        assert_eq!((c.count.min, c.count.max), (0, Some(1)), "{name}");
        assert!(old.channel(name).is_none(), "{name} is new");
    }
    // `lane` keeps its meaning; no lane-list channel, no `#n`
    assert!(new.channel("lane").is_some());
    assert!(
        new.channel("lanes").is_none()
            && new.channel("lane-label").is_none()
            && new.channel("#n").is_none()
    );
}

#[test]
fn the_older_versions_are_untouched() {
    // each older version is still there and has no channel of 1.4.0
    for key in OLDER {
        let f = forms::find_key(key).unwrap();
        for name in ["state", "assignee", "time-zone"] {
            assert!(f.channel(name).is_none(), "{key} has {name}");
        }
    }
    let counts: Vec<usize> = OLDER
        .iter()
        .map(|k| forms::find_key(k).unwrap().channels.len())
        .collect();
    assert_eq!(counts, [6, 11, 13, 18]);
}

#[test]
fn the_catalog_names_the_standards_the_repeat_rule_and_the_lane_list() {
    let text = forms::SOURCE;
    for needle in [
        "5.2.5",
        "4.7.1",
        "IANA",
        "repeats: assignee",
        "Rule \"repeated value\"",
        "rows@1.0.0",
        "lanes:<axis>:<board>",
    ] {
        assert!(text.contains(needle), "{needle}");
    }
    // the rule is text only: the wire schema carries it as a comment, the reader has no new column
    let capnp = include_str!("../../../schema/formsource.capnp");
    assert!(capnp.contains("Repeated values"));
}

#[test]
fn the_lane_list_form_is_rows_1_0_0_and_has_what_the_lane_source_needs() {
    let rows = forms::find_key("rows@1.0.0").unwrap();
    for name in ["items", "label", "order", "state"] {
        assert!(rows.channel(name).is_some(), "rows has {name}");
    }
}
