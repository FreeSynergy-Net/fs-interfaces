//! The search for canaries: the edges of the window, the two classes of markers, and no alarm on random text.
//! Pattern: none (tests). The random part of every marker is fresh in every run.
#![cfg(feature = "conformance")]
#![allow(
    clippy::assert_is_empty,
    clippy::format_collect,
    clippy::manual_assert_eq
)] // test code: the message of an assert says more than the shorter form

use fs_mcp::canary::{Canary, CanaryTable, Class, Encoding, fresh_marker, scan};
use fs_mcp::serve::base64;

fn one() -> (Canary, Vec<Canary>) {
    let mut t = CanaryTable::new();
    t.plant("vault.Entry.value");
    let c = t.all()[0].clone();
    (c.clone(), vec![c])
}

fn hex(bytes: &[u8], upper: bool) -> String {
    bytes
        .iter()
        .map(|b| {
            if upper {
                format!("{b:02X}")
            } else {
                format!("{b:02x}")
            }
        })
        .collect()
}

#[test]
fn a_piece_of_eight_is_a_hit_and_a_piece_of_seven_is_not() {
    let (c, cs) = one();
    let random = &c.marker[8..];
    for start in [0, 3, 8, 16] {
        let eight = &random[start..start + 8];
        assert!(
            !scan(&format!("<{eight}>"), &cs).is_empty(),
            "8 characters at {start}: {eight}"
        );
        let seven = &random[start..start + 7];
        assert_eq!(
            scan(&format!("<{seven}>"), &cs),
            vec![],
            "7 characters at {start}: {seven}"
        );
    }
    // the constant start is eight characters long: all of it is a hit, one less is not
    assert!(!scan("<FSCANARY>", &cs).is_empty());
    assert_eq!(scan("<FSCANAR>", &cs), vec![]);
    assert_eq!(scan("<SCANARY>", &cs), vec![]);
    // the same in hex and in base64
    let h = hex(c.marker.as_bytes(), false);
    assert!(
        !scan(&h[20..36], &cs).is_empty(),
        "16 hex digits are 8 characters"
    );
    assert_eq!(
        scan(&h[20..34], &cs),
        vec![],
        "14 hex digits are 7 characters"
    );
}

#[test]
fn a_window_of_the_shared_start_is_named_as_such_when_there_are_several_canaries() {
    let mut t = CanaryTable::new();
    t.plant("a");
    t.plant("b");
    let hits = scan("<FSCANARY>", t.all());
    assert!(hits.iter().all(|h| h.path == "(any marker)"), "{hits:?}");
    assert!(!hits.is_empty());
}

#[test]
fn nothing_to_find_in_nothing() {
    let (c, cs) = one();
    assert_eq!(scan("", &cs), vec![]);
    assert_eq!(scan(&c.marker, &[]), vec![]);
    assert_eq!(scan("short", &cs), vec![]);
    assert_eq!(scan("{}", &cs), vec![]);
    assert_eq!(scan("[\"a\",{\"b\":null}]", &cs), vec![]);
    assert_eq!(scan("%zz%4", &cs), vec![]);
}

#[test]
fn a_megabyte_of_random_text_has_no_hit() {
    // random in every run: the markers are, and so is the text
    let mut t = CanaryTable::new();
    for i in 0..40 {
        t.plant(&format!("f{i}"));
    }
    let seed = fresh_marker();
    let mut state: u64 = seed
        .bytes()
        .fold(0x9e37_79b9_7f4a_7c15_u64, |a, b| {
            a.rotate_left(7) ^ u64::from(b)
        })
        .max(1);
    let mut text = String::with_capacity(1 << 20);
    while text.len() < (1 << 20) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        text.push(char::from(32 + u8::try_from(state % 95).unwrap()));
    }
    let hits = scan(&text, t.all());
    assert_eq!(hits, vec![], "seed {seed}");
}

#[test]
fn text_that_merely_looks_alike_is_no_hit() {
    let (c, cs) = one();
    // the right length, one character wrong in every window position
    let mut broken = c.marker.clone().into_bytes();
    for i in (8..broken.len()).step_by(7) {
        broken[i] = if broken[i] == b'A' { b'B' } else { b'A' };
    }
    let broken = String::from_utf8(broken).unwrap();
    let hits = scan(&broken, &cs);
    // windows that do not cross a broken position are still real pieces of the marker: they are hits by design
    assert!(hits.iter().all(|h| h.path == c.path));
    // text about canaries, without a marker
    assert_eq!(scan("<FSCAN>: not enough!", &cs), vec![]);
}

#[test]
fn the_class_of_a_marker_is_in_the_hit_and_both_classes_are_found() {
    let mut t = CanaryTable::new();
    let s = t.plant_as("vault.value", Class::Secret);
    let n = t.plant_as("person.diagnosis", Class::Sensitive);
    assert!(s.starts_with("FSCANARY") && n.starts_with("FSSENS"));
    assert_eq!(fs_mcp::canary::class_of(&s), Some(Class::Secret));
    assert_eq!(fs_mcp::canary::class_of(&n), Some(Class::Sensitive));
    assert_eq!(fs_mcp::canary::class_of(&format!("{s}x")), None);
    let hits = scan(&format!("a {s} b {n}"), t.all());
    let classes: std::collections::BTreeSet<(String, Class)> =
        hits.iter().map(|h| (h.path.clone(), h.class)).collect();
    assert_eq!(
        classes,
        [
            ("vault.value".to_owned(), Class::Secret),
            ("person.diagnosis".to_owned(), Class::Sensitive)
        ]
        .into()
    );
    assert_eq!(t.of_class(Class::Sensitive).len(), 1);
    // a sensitive marker in the other forms too (its prefix has six characters: a window reaches into the random part)
    let hex: String = n.bytes().map(|b| format!("{b:02x}")).collect();
    assert!(
        scan(&hex, t.all())
            .iter()
            .any(|h| h.class == Class::Sensitive && h.encoding == Encoding::Hex)
    );
    assert!(
        scan(&base64::encode(n.as_bytes()), t.all())
            .iter()
            .any(|h| h.class == Class::Sensitive)
    );
    // only the shared start of the sensitive class: said once, as sensitive
    let only = scan("<FSSENS>", t.all());
    assert!(
        only.is_empty(),
        "six characters are less than a window: {only:?}"
    );
    // two markers of known content: the shared six characters plus two that differ from both
    let mk = |path: &str, fill: char| Canary {
        path: path.into(),
        marker: format!("FSSENS{}", fill.to_string().repeat(24)),
        class: Class::Sensitive,
    };
    let two = [mk("a", 'Q'), mk("b", 'R')];
    assert_eq!(
        scan("<FSSENSAB>", &two),
        vec![],
        "8 characters, but the random part differs"
    );
    let hits = scan("<FSSENSQQQQQQ>", &two);
    assert!(
        !hits.is_empty()
            && hits
                .iter()
                .all(|h| h.path == "a" && h.class == Class::Sensitive),
        "{hits:?}"
    );
}
