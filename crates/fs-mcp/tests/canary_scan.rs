//! The search for canaries: every form a marker can come back in, the edges of the window, and no alarm on random text.
//! Pattern: none (tests). The random part of every marker is fresh in every run.
#![cfg(feature = "conformance")]
#![allow(
    clippy::assert_is_empty,
    clippy::format_collect,
    clippy::manual_assert_eq
)] // test code: the message of an assert says more than the shorter form

use fs_mcp::canary::{Canary, CanaryTable, Class, Encoding, WINDOW, scan};
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
fn a_base64_character_that_only_partly_belongs_to_the_marker_does_not_count() {
    // the characters at the edge also hold bits of the bytes around the marker: only the fully determined ones are searched.
    // Seven fully determined characters and a partial one are 7 characters of the marker, so no hit. (At the start of the marker
    // the constant `FSCANARY` is: no window there is a hit on its own, see `windows_that_are_mostly_the_constant_start_are_no_hit`.)
    let (c, cs) = one();
    for before in 0..3_usize {
        let mut bytes = vec![0_u8; before];
        bytes.extend_from_slice(c.marker.as_bytes());
        let text = base64::encode(&bytes).trim_end_matches('=').to_owned();
        let first_full = (8 * before).div_ceil(6);
        let full_end = 8 * bytes.len() / 6;
        if full_end < text.len() {
            // the seven before the partial character at the end, and that character
            let piece = &text[full_end - 7..=full_end];
            assert_eq!(scan(piece, &cs), vec![], "before {before}: {piece}");
        }
        // eight fully determined characters of the random part are a hit, a shifted piece too
        for at in [first_full + 12, full_end - 8] {
            let piece = &text[at..at + 8];
            assert!(!scan(piece, &cs).is_empty(), "before {before}: {piece}");
        }
    }
}

#[test]
fn windows_that_are_mostly_the_constant_start_are_no_hit() {
    // the marker starts with the same eight characters every time: a window that holds little else must not match innocent text
    let (c, cs) = one();
    let prefix = &c.marker[..8];
    for innocent in [
        "canary-tasks",
        "the canary test",
        "fscan canary tail",
        "scanary",
        "FSCANARX",
    ] {
        assert_eq!(scan(&format!("<{innocent}>"), &cs), vec![], "{innocent}");
    }
    // the constant start with up to five of the random characters is still no hit (one piece of 8 holds at most 5 random ones)
    for k in 1..=5 {
        let piece = format!("<{}{}>", &prefix[k..], &c.marker[8..8 + k]);
        assert_eq!(scan(&piece, &cs), vec![], "{piece}");
    }
    // six random characters in a window of eight are a hit
    let piece = format!("<{}{}>", &prefix[6..], &c.marker[8..14]);
    assert!(!scan(&piece, &cs).is_empty(), "{piece}");
    // the marker that was found in an innocent answer by CI (windows, `canary-tasks` and a marker that starts with `TA`)
    let seen = Canary {
        path: "p".into(),
        marker: "FSCANARYTAPK6PL7ZIBIXDHCY3SXAXZB".into(),
        class: fs_mcp::canary::Class::Secret,
    };
    let answer = "{\"_meta\":{\"io.modelcontextprotocol/serverInfo\":{\"name\":\"canary-tasks\"},\"net.freesynergy/mcp\":{\"actor\":\"ai:canary\",\"ai\":true}}}";
    assert_eq!(scan(answer, std::slice::from_ref(&seen)), vec![]);
    // the whole constant start alone is said once
    assert!(!scan("<FSCANARY>", &cs).is_empty());
}
