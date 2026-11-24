//! The four scaffold forms (24.11.2026, ENTSCHEID-formen-scaffold): `tiles@1.1.0`, `rows@1.0.0`, `single-record@1.0.0` and `fields-in-groups@1.0.0`.
//! The channels, their arts, the required ones and the version are pinned; `tiles@1.0.0` and `members@1.0.0` stay (tags of fs-components bind them).

use fs_interfaces::forms::{self, Art, Count, Status};

const ANY: [Art; 5] = [Art::Text, Art::Number, Art::Time, Art::Flag, Art::Reference];

fn names<'a>(it: impl Iterator<Item = &'a forms::Channel>) -> Vec<&'a str> {
    it.map(|c| c.name.as_str()).collect()
}

fn pin(key: &str, required: &[&str], channels: &[(&str, &[Art])]) {
    let f = forms::find_key(key).unwrap_or_else(|| panic!("{key}"));
    assert_eq!(f.status, Status::Planned, "{key}");
    assert_eq!(names(f.required()), required, "{key}");
    for (name, arts) in channels {
        let c = f
            .channel(name)
            .unwrap_or_else(|| panic!("{key}: channel {name}"));
        assert_eq!(c.arts, *arts, "{key}: {name}");
    }
    assert_eq!(f.channels.len(), channels.len(), "{key}");
}

#[test]
fn tiles_1_1_0_adds_four_channels_and_1_0_0_stays() {
    let old = forms::find_key("tiles@1.0.0").unwrap();
    let new = forms::find_key("tiles@1.1.0").unwrap();
    assert_eq!(new.key(), "tiles@1.1.0");
    for c in &old.channels {
        let n = new
            .channel(&c.name)
            .unwrap_or_else(|| panic!("lost {}", c.name));
        assert_eq!((&n.arts, n.required, n.count), (&c.arts, c.required, c.count));
    }
    assert_eq!(new.channels.len(), old.channels.len() + 4);
    for name in ["summary", "state", "progress", "actions"] {
        let c = new.channel(name).unwrap();
        assert!(!c.required, "{name}");
    }
    assert_eq!(new.channel("actions").unwrap().arts, [Art::Command]);
    assert_eq!(
        new.channel("actions").unwrap().count,
        Count { min: 0, max: None }
    );
    assert_eq!(names(new.required()), ["items", "label"]);
}

#[test]
fn rows_has_typed_columns_and_two_required_channels() {
    pin(
        "rows@1.0.0",
        &["items", "label"],
        &[
            ("items", &[Art::Records]),
            ("label", &[Art::Text]),
            ("columns", &ANY),
            ("preview", &[Art::Reference]),
            ("state", &[Art::Text]),
            ("order", &[Art::Number]),
            ("open", &[Art::Command]),
            ("actions", &[Art::Command]),
        ],
    );
    let f = forms::find_key("rows@1.0.0").unwrap();
    assert_eq!(
        f.channel("columns").unwrap().count,
        Count { min: 0, max: None }
    );
}

#[test]
fn single_record_is_one_item_with_fields_and_actions() {
    pin(
        "single-record@1.0.0",
        &["item", "label"],
        &[
            ("item", &[Art::Records]),
            ("label", &[Art::Text]),
            ("preview", &[Art::Reference]),
            ("summary", &[Art::Text]),
            ("content", &[Art::Text]),
            ("state", &[Art::Text]),
            ("progress", &[Art::Number]),
            ("fields", &ANY),
            ("actions", &[Art::Command]),
        ],
    );
}

#[test]
fn fields_in_groups_has_sixteen_channels_and_basic_is_a_channel() {
    pin(
        "fields-in-groups@1.0.0",
        &["items", "label", "basic", "value"],
        &[
            ("items", &[Art::Records]),
            ("label", &[Art::Text]),
            ("basic", &[Art::Text]),
            ("value", &ANY),
            ("group", &[Art::Text]),
            ("hint", &[Art::Text]),
            ("default", &ANY),
            ("choices", &[Art::Text]),
            ("min", &[Art::Number]),
            ("max", &[Art::Number]),
            ("layer", &[Art::Text]),
            ("locked", &[Art::Flag]),
            ("lock-reason", &[Art::Text]),
            ("order", &[Art::Number]),
            ("change", &[Art::Command]),
            ("reset", &[Art::Command]),
        ],
    );
}

#[test]
fn old_forms_stay_and_members_is_marked_frozen() {
    for key in ["tiles@1.0.0", "members@1.0.0"] {
        assert!(forms::find_key(key).is_some(), "{key}");
    }
    assert!(forms::SOURCE.contains("FROZEN (24.11.2026): superseded by `rows@1.0.0`"));
}

/// The standards named in the comments were read in the texts (24.11.2026); the names stay in the catalog.
#[test]
fn the_standards_of_the_scaffold_forms_are_named() {
    for needle in [
        "JSON Schema Validation, draft 2020-12",
        "FHIR R5 Questionnaire",
        "CSVW",
        "ActivityStreams 2.0 Vocabulary",
        "AppStream",
    ] {
        assert!(forms::SOURCE.contains(needle), "{needle}");
    }
}
