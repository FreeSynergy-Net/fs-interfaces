//! S1 E3 (SCHNITT-move-position): `cards-in-columns@1.2.0` is appended; the place of a card is never an integer.
//! `order` stays a read-only number, `move` writes the neighbours `after` and `before` (record ids).

use fs_interfaces::forms::{self, Art};

fn writes(form: &forms::Form, command: &str) -> Vec<(String, bool)> {
    form.channel(command)
        .unwrap()
        .writes
        .as_ref()
        .unwrap()
        .iter()
        .map(|w| (w.channel.clone(), w.required))
        .collect()
}

#[test]
fn version_1_2_0_adds_the_neighbour_channels_and_keeps_1_1_0() {
    let old = forms::find_key("cards-in-columns@1.1.0").unwrap();
    let new = forms::find_key("cards-in-columns@1.2.0").unwrap();
    for c in &old.channels {
        let n = new.channel(&c.name).unwrap();
        assert_eq!(
            (&c.arts, c.required, c.count),
            (&n.arts, n.required, n.count),
            "{}",
            c.name
        );
    }
    assert_eq!(new.channels.len(), old.channels.len() + 2);
    for name in ["after", "before"] {
        let c = new.channel(name).unwrap();
        assert_eq!(c.arts, [Art::Reference], "{name}");
        assert!(!c.required, "{name}");
        assert_eq!(c.count.max, Some(1), "{name}");
        assert!(old.channel(name).is_none(), "{name} is new");
    }
}

#[test]
fn move_writes_the_neighbours_and_never_the_order() {
    let old = forms::find_key("cards-in-columns@1.1.0").unwrap();
    let new = forms::find_key("cards-in-columns@1.2.0").unwrap();
    let s = |n: &str, r| (n.to_owned(), r);
    assert_eq!(
        writes(new, "move"),
        [
            s("column", true),
            s("after", false),
            s("before", false),
            s("lane", false)
        ]
    );
    // order is still there, as a number to read; no channel writes it, and no key channel exists
    assert_eq!(new.channel("order").unwrap().arts, [Art::Number]);
    assert!(
        new.channels
            .iter()
            .flat_map(|c| c.writes.iter().flatten())
            .all(|w| w.channel != "order")
    );
    assert!(new.channel("order-key").is_none() && new.channel("index").is_none());
    // the old line is untouched
    assert!(writes(old, "move").iter().any(|(n, _)| n == "order"));
    assert_eq!(writes(new, "create"), writes(old, "create"));
}
