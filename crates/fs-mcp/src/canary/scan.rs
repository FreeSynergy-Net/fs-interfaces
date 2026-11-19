//! The search: does a text hold a canary, in whatever form it was written?
//!
//! Pattern: Specification — `scan` is a pure predicate over a text and the canaries; it normalises the text (JSON strings
//! unescaped, `%XX` decoded, lower case, only letters and digits) and looks up every window of [`WINDOW`] characters of
//! every form of every marker (raw, hex, base64 at three byte offsets, URL-safe base64). A window of 8 characters is enough for
//! the forms that use many symbols: any longer piece of a marker holds one. Hex has only 16 symbols, so its window is 16
//! characters (8 bytes), or a hash in the output would be an alarm. The random part of a marker is 24 base32 characters, so a window of the random part
//! is one in about 10^12 by chance (a calculation; the tests run a megabyte of random text without a hit).

use std::collections::{BTreeSet, HashMap};

use super::marker::{Canary, Class};
use super::views::views;
use crate::serve::base64;

/// The length of the windows. A leaked piece of this length or longer is a hit, a shorter one is not (hex: twice as long).
pub const WINDOW: usize = 8;

/// The window of a form: 16 hex digits are 8 bytes, the same strength as 8 characters of base32 or base64.
fn window(enc: Encoding) -> usize {
    if enc == Encoding::Hex {
        2 * WINDOW
    } else {
        WINDOW
    }
}

/// How the marker was written where it was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Encoding {
    /// As it is.
    Raw,
    /// Only in the other letter case.
    CaseFolded,
    /// Hexadecimal.
    Hex,
    /// Base64 (standard alphabet), the marker at byte offset 0, 1 or 2 of what was encoded.
    Base64(u8),
    /// Base64 (URL-safe alphabet), the marker at byte offset 0, 1 or 2.
    Base64Url(u8),
    /// With other characters between its letters and digits.
    Separated,
}

impl std::fmt::Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Raw => f.write_str("raw"),
            Self::CaseFolded => f.write_str("case-folded"),
            Self::Hex => f.write_str("hex"),
            Self::Base64(o) => write!(f, "base64+{o}"),
            Self::Base64Url(o) => write!(f, "base64url+{o}"),
            Self::Separated => f.write_str("separated"),
        }
    }
}

/// A canary that was found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    /// The field the marker was planted in; `(any marker)` for a window of the constant prefix, which belongs to all.
    pub path: String,
    /// The marker (or its prefix for the shared window).
    pub marker: String,
    /// The class of the marker: a secret (never out) or sensitive data (out only with a release per receiver).
    pub class: Class,
    /// The form it was found in.
    pub encoding: Encoding,
    /// The view of the text it was found in: `text`, `json-string`, `percent-decoded` or both of the last two.
    pub found_in: String,
    /// The position in that view (in bytes).
    pub offset: usize,
}

/// The path of a hit on a window that belongs to every marker.
pub const ANY: &str = "(any marker)";

type Table = HashMap<Vec<u8>, Vec<(usize, Encoding)>>;

fn grams(table: &mut Table, canary: usize, enc: Encoding, form: &str) {
    for w in form.as_bytes().windows(window(enc)) {
        let list = table.entry(w.to_vec()).or_default();
        if !list.contains(&(canary, enc)) {
            list.push((canary, enc));
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// The part of the base64 text of `marker` that does not depend on the bytes around it, for the marker at byte offset
/// `o` (0, 1 or 2) of what was encoded: leading characters that mix in bytes before it and trailing ones that mix in bytes
/// after it are cut off. No padding.
fn base64_core(marker: &str, o: usize, url: bool) -> String {
    let mut bytes = vec![0_u8; o];
    bytes.extend_from_slice(marker.as_bytes());
    let text = base64::encode(&bytes);
    let from = (8 * o).div_ceil(6);
    let to = 8 * bytes.len() / 6;
    let core = &text[from..to];
    if url {
        core.replace('+', "-").replace('/', "_")
    } else {
        core.to_owned()
    }
}

/// The tables of one run, one per view of the text and window length. Hex is searched in lower case only (the text is
/// lowered too), so both cases are found.
struct Tables {
    exact: Table,
    folded: Table,
    folded_hex: Table,
    alnum: Table,
}

fn tables(canaries: &[Canary]) -> Tables {
    let mut t = Tables {
        exact: Table::new(),
        folded: Table::new(),
        folded_hex: Table::new(),
        alnum: Table::new(),
    };
    for (i, c) in canaries.iter().enumerate() {
        let m = c.marker.as_str();
        grams(&mut t.exact, i, Encoding::Raw, m);
        grams(&mut t.folded, i, Encoding::CaseFolded, &m.to_lowercase());
        grams(&mut t.alnum, i, Encoding::Separated, &m.to_lowercase());
        grams(&mut t.folded_hex, i, Encoding::Hex, &hex(m.as_bytes()));
        for o in 0..3_u8 {
            grams(
                &mut t.exact,
                i,
                Encoding::Base64(o),
                &base64_core(m, usize::from(o), false),
            );
            grams(
                &mut t.exact,
                i,
                Encoding::Base64Url(o),
                &base64_core(m, usize::from(o), true),
            );
        }
    }
    t
}

fn alnum_lower(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn look(
    table: &Table,
    len: usize,
    text: &str,
    label: &str,
    canaries: &[Canary],
    seen: &mut BTreeSet<(String, Encoding)>,
    out: &mut Vec<Hit>,
) {
    for (offset, w) in text.as_bytes().windows(len).enumerate() {
        let Some(list) = table.get(w) else { continue };
        let distinct: BTreeSet<usize> = list.iter().map(|(c, _)| *c).collect();
        for &(c, enc) in list {
            // a window of the constant prefix belongs to every marker: say so once, instead of naming a field at random
            let (path, marker, class) = if distinct.len() > 1 {
                // the start every marker of a class shares: no field can be named; the worst class among them counts
                let worst = canaries[*distinct.iter().next().unwrap_or(&0)].class;
                (ANY.to_owned(), worst.prefix().to_owned(), worst)
            } else {
                (
                    canaries[c].path.clone(),
                    canaries[c].marker.clone(),
                    canaries[c].class,
                )
            };
            if seen.insert((path.clone() + "\u{0}" + &marker, enc)) {
                out.push(Hit {
                    path,
                    marker,
                    class,
                    encoding: enc,
                    found_in: label.to_owned(),
                    offset,
                });
            }
        }
    }
}

/// Every canary that `haystack` holds, in any of the forms: raw, in another letter case, hex, base64 (three offsets, standard
/// and URL-safe), `%XX`-coded, inside an escaped JSON string, or with separators between its characters. At least
/// [`WINDOW`] characters of a marker must be there; a shorter piece is no hit. One hit per canary and form.
#[must_use]
pub fn scan(haystack: &str, canaries: &[Canary]) -> Vec<Hit> {
    if haystack.len() < WINDOW || canaries.is_empty() {
        return Vec::new();
    }
    let t = tables(canaries);
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (label, text) in views(haystack) {
        let lower = text.to_lowercase();
        let alnum = alnum_lower(&text);
        let mut go = |table: &Table, len: usize, view: &str| {
            look(table, len, view, label, canaries, &mut seen, &mut out);
        };
        go(&t.exact, WINDOW, &text);
        go(&t.folded, WINDOW, &lower);
        go(&t.folded_hex, 2 * WINDOW, &lower);
        go(&t.alnum, WINDOW, &alnum);
    }
    // a window of the shared start is only worth a line when no marker was found on its own (a leaked marker holds it too)
    if out.iter().any(|h| h.path != ANY) {
        out.retain(|h| h.path != ANY);
    }
    out.sort_by(|a, b| (&a.path, a.encoding, &a.found_in).cmp(&(&b.path, b.encoding, &b.found_in)));
    out
}
