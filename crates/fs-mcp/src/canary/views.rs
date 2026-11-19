//! The views of a text that are searched: the text itself, every string of the JSON in it (unescaped, nested JSON strings
//! too), and each of them `%XX`-decoded.
//!
//! Pattern: none (pure functions over text).

use serde_json::Value;

fn percent_decode(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hexval = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hexval(b[i + 1]), hexval(b[i + 2]))
        {
            out.push(u8::try_from(h * 16 + l).unwrap_or(b'?'));
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Every string of a JSON value (keys too), and, where a string is JSON itself, the strings of that, a few levels deep.
fn json_strings(v: &Value, depth: u32, out: &mut Vec<String>) {
    match v {
        Value::String(s) => {
            out.push(s.clone());
            if depth < 4
                && (s.starts_with('{') || s.starts_with('['))
                && let Ok(inner) = serde_json::from_str::<Value>(s)
            {
                json_strings(&inner, depth + 1, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| json_strings(x, depth, out)),
        Value::Object(m) => {
            for (k, x) in m {
                out.push(k.clone());
                json_strings(x, depth, out);
            }
        }
        _ => {}
    }
}

/// The views of the haystack that are searched: the text itself, every JSON string in it, and the `%XX`-decoded form of each.
pub(super) fn views(haystack: &str) -> Vec<(&'static str, String)> {
    let mut v = vec![("text", haystack.to_owned())];
    let mut strings = Vec::new();
    match serde_json::from_str::<Value>(haystack) {
        Ok(j) => json_strings(&j, 0, &mut strings),
        Err(_) => {
            for line in haystack
                .lines()
                .map(str::trim)
                .filter(|l| l.starts_with('{') || l.starts_with('['))
            {
                if let Ok(j) = serde_json::from_str::<Value>(line) {
                    json_strings(&j, 0, &mut strings);
                }
            }
        }
    }
    let in_json = !strings.is_empty();
    v.extend(strings.into_iter().map(|s| ("json-string", s)));
    if haystack.contains('%') || v.iter().any(|(_, s)| s.contains('%')) {
        let decoded: Vec<_> = v
            .iter()
            .filter(|(_, s)| s.contains('%'))
            .map(|(label, s)| {
                (
                    if *label == "text" || !in_json {
                        "percent-decoded"
                    } else {
                        "json-string+percent-decoded"
                    },
                    percent_decode(s),
                )
            })
            .collect();
        v.extend(decoded);
    }
    v
}
