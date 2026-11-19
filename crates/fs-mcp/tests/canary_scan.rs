//! The search for canaries: every form a marker can come back in, the edges of the window, and no alarm on random text.
//! Pattern: none (tests). The random part of every marker is fresh in every run.
#![cfg(feature = "conformance")]

use fs_mcp::canary::{Canary, CanaryTable, Class, Encoding, WINDOW, fresh_marker, scan};
use fs_mcp::serve::base64;
use serde_json::json;

fn one() -> (Canary, Vec<Canary>) {
    let mut t = CanaryTable::new();
    t.plant("vault.Entry.value");
    let c = t.all()[0].clone();
    (c.clone(), vec![c])
}

fn encodings(h: &str, cs: &[Canary]) -> Vec<Encoding> {
    scan(h, cs).into_iter().map(|x| x.encoding).collect()
}

#[test]
fn the_window_is_eight() {
    assert_eq!(WINDOW, 8);
}

#[test]
fn raw_and_in_the_middle_of_a_sentence() {
    let (c, cs) = one();
    for h in [
        c.marker.clone(),
        format!("the value is {} and nothing else", c.marker),
        format!("{}\n{}", "x".repeat(5000), c.marker),
    ] {
        let hits = scan(&h, &cs);
        assert!(
            hits.iter()
                .any(|x| x.encoding == Encoding::Raw && x.path == c.path && x.marker == c.marker),
            "{h}"
        );
    }
}

#[test]
fn a_hit_names_the_field_and_the_form() {
    let mut t = CanaryTable::new();
    let a = t.plant("a.first");
    let b = t.plant("b.second");
    let cs = t.all().to_vec();
    let hits = scan(&format!("only {b} is here"), &cs);
    assert!(
        !hits.is_empty() && hits.iter().all(|h| h.path == "b.second" && h.marker == b),
        "{hits:?}"
    );
    let hits = scan(&format!("{a} {b}"), &cs);
    let paths: std::collections::BTreeSet<&str> = hits.iter().map(|h| h.path.as_str()).collect();
    assert_eq!(paths, ["a.first", "b.second"].into());
}

#[test]
fn json_escaped_forms_are_found() {
    let (c, cs) = one();
    // every character as a \uXXXX escape, in a string value and in a key
    let escaped: String = c
        .marker
        .chars()
        .map(|ch| format!("\\u{:04x}", u32::from(ch)))
        .collect();
    for h in [
        format!("{{\"a\":\"{escaped}\"}}"),
        format!("{{\"{escaped}\":1}}"),
        format!("{{\"a\":[1,\"x{escaped}y\"]}}"),
    ] {
        let hits = scan(&h, &cs);
        assert!(
            hits.iter()
                .any(|x| x.found_in.starts_with("json-string") && x.encoding == Encoding::Raw),
            "{h}: {hits:?}"
        );
    }
    // a JSON text inside a JSON string (the text block of a tool result), with the marker inside it escaped twice
    let inner = json!({"secret": c.marker}).to_string();
    let outer = json!({"content": [{"type": "text", "text": inner}]}).to_string();
    assert!(
        outer.contains("\\\""),
        "the inner text is escaped in the outer"
    );
    assert!(!scan(&outer, &cs).is_empty());
    let inner_escaped = format!("{{\"x\":\"{escaped}\"}}");
    let outer = json!({"text": inner_escaped}).to_string();
    assert!(
        scan(&outer, &cs)
            .iter()
            .any(|x| x.encoding == Encoding::Raw),
        "{outer}"
    );
    // several lines, one of them JSON
    let lines = format!("log line\n{{\"a\":\"{escaped}\"}}\nlast");
    assert!(!scan(&lines, &cs).is_empty());
}

#[test]
fn hex_in_both_cases() {
    let (c, cs) = one();
    for hex in [
        hex(c.marker.as_bytes(), false),
        hex(c.marker.as_bytes(), true),
    ] {
        assert!(encodings(&hex, &cs).contains(&Encoding::Hex), "{hex}");
        assert!(encodings(&format!("dump: {hex} end"), &cs).contains(&Encoding::Hex));
    }
    let mixed: String = hex(c.marker.as_bytes(), false)
        .chars()
        .enumerate()
        .map(|(i, ch)| {
            if i % 2 == 0 {
                ch.to_ascii_uppercase()
            } else {
                ch
            }
        })
        .collect();
    assert!(encodings(&mixed, &cs).contains(&Encoding::Hex), "{mixed}");
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
fn base64_at_all_three_offsets_standard_and_url_safe() {
    let (c, cs) = one();
    for before in 0..3_usize {
        for after in 0..3_usize {
            let mut bytes = b"\x01\xfe\x7f"[..before].to_vec();
            bytes.extend_from_slice(c.marker.as_bytes());
            bytes.extend_from_slice(&b"\xff\xfb\xef"[..after]);
            let std = base64::encode(&bytes);
            let url = std
                .replace('+', "-")
                .replace('/', "_")
                .trim_end_matches('=')
                .to_owned();
            let want = u8::try_from(before % 3).unwrap();
            assert!(
                encodings(&std, &cs).contains(&Encoding::Base64(want)),
                "std before {before} after {after}: {std}"
            );
            assert!(
                encodings(&url, &cs).contains(&Encoding::Base64Url(want)),
                "url before {before} after {after}: {url}"
            );
            // inside a longer text, and unpadded
            assert!(
                encodings(&format!("Authorization: Basic {std}; next"), &cs)
                    .contains(&Encoding::Base64(want))
            );
            assert!(encodings(std.trim_end_matches('='), &cs).contains(&Encoding::Base64(want)));
        }
    }
}

#[test]
fn url_safe_base64_that_really_uses_the_other_alphabet() {
    // a marker made so that its base64 has `+` and `/`: the URL-safe form differs from the standard one
    let c = Canary {
        path: "p".into(),
        marker: "FSCANARY>>>???~~~>>>???~~~ABCDEFGH".into(),
        class: Class::Secret,
    };
    let std = base64::encode(c.marker.as_bytes());
    assert!(std.contains('+') || std.contains('/'), "{std}");
    let url = std.replace('+', "-").replace('/', "_");
    assert!(url != std);
    assert!(encodings(&url, std::slice::from_ref(&c)).contains(&Encoding::Base64Url(0)));
    assert!(encodings(&std, std::slice::from_ref(&c)).contains(&Encoding::Base64(0)));
}

#[test]
fn percent_coded_in_a_url_and_in_json() {
    let (c, cs) = one();
    for upper in [true, false] {
        let coded: String = c
            .marker
            .bytes()
            .map(|b| {
                if upper {
                    format!("%{b:02X}")
                } else {
                    format!("%{b:02x}")
                }
            })
            .collect();
        let hits = scan(&format!("https://example.org/?q={coded}&x=1"), &cs);
        assert!(
            hits.iter()
                .any(|x| x.found_in.contains("percent-decoded") && x.encoding == Encoding::Raw),
            "{hits:?}"
        );
        let j = json!({"url": format!("/a?{coded}")}).to_string();
        assert!(
            scan(&j, &cs)
                .iter()
                .any(|x| x.found_in.contains("percent-decoded"))
        );
    }
}

#[test]
fn other_case_and_separators_between_the_characters() {
    let (c, cs) = one();
    assert!(encodings(&c.marker.to_lowercase(), &cs).contains(&Encoding::CaseFolded));
    for sep in ["-", " ", ".", "_", "\n", "\t", ": ", "</>"] {
        let spaced: Vec<String> = c.marker.chars().map(String::from).collect();
        let h = format!("x {} y", spaced.join(sep));
        assert!(
            encodings(&h, &cs).contains(&Encoding::Separated),
            "separator {sep:?}"
        );
        let groups: Vec<String> = c
            .marker
            .as_bytes()
            .chunks(4)
            .map(|g| String::from_utf8_lossy(g).into_owned())
            .collect();
        assert!(
            encodings(&groups.join(sep), &cs).contains(&Encoding::Separated),
            "groups with {sep:?}"
        );
    }
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
    assert_eq!(
        scan("the canary test plants canaries; FSCAN is not enough", &cs),
        vec![]
    );
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
    let mut two = CanaryTable::new();
    two.plant_as("a", Class::Sensitive);
    two.plant_as("b", Class::Sensitive);
    assert_eq!(
        scan("<FSSENSAB>", two.all()),
        vec![],
        "8 characters, but the marker's random part differs"
    );
}
