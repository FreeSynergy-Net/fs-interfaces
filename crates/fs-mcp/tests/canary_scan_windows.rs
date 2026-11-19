//! The windows of the search: which pieces of a marker count (enough of the random part), which do not (the constant start,
//! the characters at the edge of a base64 text that also hold other bytes). Pattern: none (tests). Fresh markers in every run.
#![cfg(feature = "conformance")]
#![allow(
    clippy::assert_is_empty,
    clippy::format_collect,
    clippy::manual_assert_eq
)] // test code: the message of an assert says more

use fs_mcp::canary::{Canary, CanaryTable, scan};
use fs_mcp::serve::base64;

fn one() -> (Canary, Vec<Canary>) {
    let mut t = CanaryTable::new();
    t.plant("vault.Entry.value");
    let c = t.all()[0].clone();
    (c.clone(), vec![c])
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
    // the same in hex: sixteen digits that are mostly the start of the marker are no hit, twelve random digits of sixteen are
    let h: String = c.marker.bytes().map(|b| format!("{b:02x}")).collect();
    for k in 1..=10 {
        let piece = &h[16 - (16 - k)..16 + k]; // (16 - k) digits of the constant start, then k random ones
        assert_eq!(scan(piece, &cs), vec![], "{k} random digits: {piece}");
    }
    assert!(
        !scan(&h[16 - 4..16 + 12], &cs).is_empty(),
        "12 random digits of 16"
    );
    // the same in base64, at every offset: the first windows of the encoded marker are mostly its constant start
    for before in 0..3_usize {
        let mut bytes = vec![0_u8; before];
        bytes.extend_from_slice(c.marker.as_bytes());
        let text = base64::encode(&bytes);
        let from = (8 * before).div_ceil(6);
        for s in 0..=6 {
            let piece = &text[from + s..from + s + 8];
            assert_eq!(
                scan(piece, &cs),
                vec![],
                "before {before}, window {s}: {piece}"
            );
        }
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
