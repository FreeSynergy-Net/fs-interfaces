//! The values of the standard channels the fake serves for card `c1`.
//!
//! Pattern: Fake Object (fixed example values; `LaxValues` gives wrong ones on purpose, `LaxRepeat` a repeated value at a channel that may not repeat).

use crate::Mode;

/// The values of the eight standard channels of card `c1`. `assignee` carries TWO values (the one channel that repeats).
/// `LaxValues` breaks each channel on purpose; `LaxRepeat` keeps them right but repeats `priority`.
pub(crate) fn standard_values(mode: Mode) -> Vec<(&'static str, String)> {
    let lax = mode == Mode::LaxValues;
    let pick = |good: &str, bad: &str| (if lax { bad } else { good }).to_owned();
    let mut v = vec![
        ("description", pick("A text", "")),
        ("color", pick("#1e90ff", "#12")),
        ("estimated", pick("PT1H30M", "1h")),
        ("column-limit", pick("3", "0")),
        ("priority", pick("2", "10")),
        // RFC 8984 5.2.5 `progress`: the word as written; a made-up word is wrong.
        ("state", pick("in-process", "done")),
        // RFC 8984 4.7.1 `timeZone`: an IANA name.
        ("time-zone", pick("Europe/Berlin", "Mars/Olympus")),
        // A reference (IRI); the people at one card repeat the value.
        (
            "assignee",
            pick("https://freesynergy.net/actor/anna", "anna"),
        ),
        ("assignee", pick("mailto:bob@example.org", "bob")),
    ];
    if mode == Mode::LaxRepeat {
        v.push(("priority", "3".to_owned()));
    }
    v
}
