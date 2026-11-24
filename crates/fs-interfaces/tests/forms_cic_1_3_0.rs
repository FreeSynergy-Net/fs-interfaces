//! CIC-A: `cards-in-columns@1.3.0` is appended: five optional channels in standard values (RFC 8984); 1.2.0 stays as it was.

use fs_interfaces::forms::{self, Art};

#[test]
fn version_1_3_0_adds_five_optional_channels_and_keeps_1_2_0() {
    let old = forms::find_key("cards-in-columns@1.2.0").unwrap();
    let new = forms::find_key("cards-in-columns@1.3.0").unwrap();
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
        ("description", Art::Text),
        ("color", Art::Text),
        ("estimated", Art::Text),
        ("column-limit", Art::Number),
        ("priority", Art::Number),
    ];
    assert_eq!(new.channels.len(), old.channels.len() + added.len());
    for (name, art) in added {
        let c = new.channel(name).unwrap();
        assert_eq!(c.arts, [art], "{name}");
        assert!(!c.required, "{name}");
        assert_eq!((c.count.min, c.count.max), (0, Some(1)), "{name}");
        assert!(old.channel(name).is_none(), "{name} is new");
    }
    // no lane channel, no `#n`
    assert!(new.channel("lane-label").is_none() && new.channel("#n").is_none());
}

#[test]
fn the_catalog_names_the_standard_sections_and_says_the_limit_has_none() {
    let text = forms::SOURCE;
    for needle in ["4.2.2", "4.2.11", "5.2.3", "1.4.6", "4.4.1", "NO standard"] {
        assert!(text.contains(needle), "{needle}");
    }
}
