//! V1: `single-record@1.2.0` is appended: the channels of 1.1.0 and the marker `repeats: fields`; 1.0.0 and 1.1.0 stay as they were.
//! Pattern: Projection – the catalog names the channels in standard values; sections and labels belong to the type catalog of the module.

use fs_interfaces::forms::{self, Art};

#[test]
fn version_1_2_0_has_the_channels_of_1_1_0_unchanged() {
    let old = forms::find_key("single-record@1.1.0").unwrap();
    let new = forms::find_key("single-record@1.2.0").unwrap();
    assert_eq!(old.channels.len(), 11);
    assert_eq!(new.channels.len(), old.channels.len());
    for c in &old.channels {
        let n = new.channel(&c.name).unwrap();
        assert_eq!(
            (&c.arts, c.required, c.count, &c.writes),
            (&n.arts, n.required, n.count, &n.writes),
            "{}",
            c.name
        );
    }
    let fields = new.channel("fields").unwrap();
    assert_eq!(
        fields.arts,
        [Art::Text, Art::Number, Art::Time, Art::Flag, Art::Reference]
    );
    assert_eq!((fields.count.min, fields.count.max), (0, None));
    assert!(new.channel("item").unwrap().required && new.channel("label").unwrap().required);
}

#[test]
fn the_older_versions_are_untouched_and_findable() {
    for (key, n) in [("single-record@1.0.0", 9), ("single-record@1.1.0", 11)] {
        assert_eq!(forms::find_key(key).unwrap().channels.len(), n, "{key}");
    }
    assert!(
        forms::find_key("single-record@1.0.0")
            .unwrap()
            .channel("media-type")
            .is_none()
    );
    assert!(forms::find_key("single-record@1.2.0").is_some());
    assert!(forms::find_key("single-record@1.3.0").is_none());
}

#[test]
fn the_catalog_marks_fields_as_repeating_and_the_rule_names_both_forms() {
    let text = forms::SOURCE;
    let start = text.find("# single-record@1.2.0").unwrap();
    let block = &text[start..];
    for needle in [
        "repeats: fields",
        "Rule \"repeated value\"",
        "cards-in-columns@1.4.0",
    ] {
        assert!(block.contains(needle), "{needle}");
    }
    // 1.0.0 and 1.1.0 carry no marker
    assert!(!text[..start].contains("repeats: fields"));
    let capnp = include_str!("../../../schema/formsource.capnp");
    assert!(capnp.contains("single-record@1.2.0"));
}
