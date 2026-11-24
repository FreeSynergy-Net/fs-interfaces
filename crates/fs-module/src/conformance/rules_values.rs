//! Rule 22 of `module_v1`: values in the form of the standard (`description`, `color`, `estimated`, `column-limit`, `priority`, `state`, `time-zone`, `assignee`).
//!
//! Pattern: Template Method (a fixed rule; the fixture says which field of which form carries which kind of value).

use fs_jscalendar::time::Zone;
use fs_types::{BasicValue, Duration};

use super::css_names::NAMES;
use super::cx::Cx;
use crate::ValueKind;

/// JSCalendar `color` (RFC 8984 section 4.2.11): a CSS3 colour name or `#rgb` / `#rrggbb`.
fn is_color(t: &str) -> bool {
    match t.strip_prefix('#') {
        Some(hex) => matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit()),
        None => NAMES.contains(&t.to_ascii_lowercase().as_str()),
    }
}

/// The five words of RFC 8984 section 5.2.5 `progress`, as written.
const PROGRESS: [&str; 5] = [
    "needs-action",
    "in-process",
    "completed",
    "failed",
    "cancelled",
];

/// RFC 8984 section 5.2.5: one of the five words, or a vendor word `<domain>:<name>` (a dot in the domain, no blanks).
fn is_state(t: &str) -> bool {
    PROGRESS.contains(&t)
        || t.split_once(':').is_some_and(|(d, n)| {
            d.contains('.')
                && !n.is_empty()
                && !t.chars().any(|c| c.is_whitespace() || c.is_control())
        })
}

/// RFC 3987 shape of an absolute IRI: `scheme ":" rest` (scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )), rest not empty, no blank or control character.
fn is_iri(t: &str) -> bool {
    let Some((scheme, rest)) = t.split_once(':') else {
        return false;
    };
    let mut c = scheme.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic())
        && c.all(|x| x.is_ascii_alphanumeric() || matches!(x, '+' | '-' | '.'))
        && !rest.is_empty()
        && !t.chars().any(|x| x.is_whitespace() || x.is_control())
}

/// Whether the text is a correct value of this kind (an unknown kind, added later, is NOT correct: fail closed).
#[allow(unreachable_patterns)] // `ValueKind` is non-exhaustive for OTHER crates; the wildcard arm is the fail-closed rule for a kind appended later.
pub(crate) fn is_valid(kind: ValueKind, t: &str) -> bool {
    match kind {
        ValueKind::Description => !t.is_empty(),
        ValueKind::Color => is_color(t),
        // RFC 8984 sections 5.2.3 and 1.4.6: an ISO 8601 duration (no spaces, no sign); `fs-types` reads the subset.
        ValueKind::Estimated => t == t.trim() && Duration::parse(t).is_ok(),
        // The WIP limit of a column: a whole number >= 1 (empty = no limit = no value at all).
        ValueKind::ColumnLimit => t.parse::<u32>().is_ok_and(|n| n >= 1),
        // RFC 8984 section 4.4.1: an integer 0..9.
        ValueKind::Priority => t.parse::<u8>().is_ok_and(|n| n <= 9),
        ValueKind::State => is_state(t),
        // RFC 8984 section 4.7.1: an IANA name known to the tz database (`fs-jscalendar` reads it; no parser of our own); a custom `/...` id is refused there.
        ValueKind::TimeZone => t == t.trim() && Zone::parse(t).is_ok(),
        ValueKind::Assignee => is_iri(t),
        _ => false,
    }
}

/// Rule 22: every value read at a declared value field is in standard form.
pub(crate) async fn standard_values(cx: &mut Cx<'_>) {
    let fields = cx.fx.value_fields();
    if fields.is_empty() {
        return cx.skip("22 standard values (the fixture has no value fields)");
    }
    let forms = cx.fx.forms();
    for v in fields {
        let scope = forms
            .iter()
            .find(|f| f.key == v.form)
            .map(|f| f.scope.clone())
            .unwrap_or_default();
        let rows = cx.walk(&v.form, &scope, 100).await.unwrap_or_default();
        // One value per card, except where the kind repeats (the people at a card): a second value elsewhere is wrong.
        let once = v.kind.repeats()
            || rows
                .iter()
                .all(|r| r.values.iter().filter(|(p, _)| p == &v.path).count() <= 1);
        let values: Vec<&str> = rows
            .iter()
            .flat_map(|r| &r.values)
            .filter(|(p, _)| p == &v.path)
            .map(|(_, t)| t.as_str())
            .collect();
        cx.ensure(
            !values.is_empty() && values.iter().all(|t| is_valid(v.kind, t)),
            &format!(
                "22 {} has a value and every value is a {:?} in the form of the standard",
                v.path, v.kind
            ),
        );
        cx.ensure(
            once,
            &format!(
                "22 {} has at most one value per card (only a reference to a person repeats)",
                v.path
            ),
        );
    }
}
