//! The canaries: unique markers that are planted into fields a program must never give out.
//!
//! Pattern: Value object — a [`Canary`] is a field path and its marker. A marker is `FSCANARY` and 24 characters of
//! base32 (120 bits), fresh for every run and every field, so a hit names the field and cannot be a coincidence.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

/// The constant start of the marker of a secret.
pub const PREFIX: &str = "FSCANARY";
/// The constant start of the marker of sensitive data.
pub const PREFIX_SENSITIVE: &str = "FSSENS";

/// What a marker stands for. The order is the order of severity: a secret is the worst.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    /// Credentials, keys, tokens: never out, not even with a release.
    Secret,
    /// Health data, marked contacts, DPV categories: out only with an explicit release for each receiver.
    Sensitive,
}

impl Class {
    /// The constant start of the markers of this class.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Secret => PREFIX,
            Self::Sensitive => PREFIX_SENSITIVE,
        }
    }

    /// The name in reports.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Secret => "secret",
            Self::Sensitive => "sensitive",
        }
    }
}
/// How many random characters follow the prefix.
pub const RANDOM_LEN: usize = 24;

const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// 8 bytes the process cannot predict: `RandomState` is seeded by the operating system, and the counter and the clock
/// make two calls differ. (A test marker, not key material.)
fn entropy() -> u64 {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut h = RandomState::new().build_hasher();
    h.write_u64(n);
    h.write_u128(t);
    h.finish()
}

/// A fresh marker of a secret: `FSCANARY` and 24 base32 characters.
#[must_use]
pub fn fresh_marker() -> String {
    fresh_marker_of(Class::Secret)
}

/// A fresh marker of a class: its prefix and 24 base32 characters.
#[must_use]
pub fn fresh_marker_of(class: Class) -> String {
    let mut bits: u128 = (u128::from(entropy()) << 64) | u128::from(entropy());
    let mut out = String::from(class.prefix());
    for _ in 0..RANDOM_LEN {
        out.push(char::from(BASE32[(bits & 31) as usize]));
        bits >>= 5;
    }
    out
}

/// The class of a text that is a marker of the form above, or `None`.
#[must_use]
pub fn class_of(text: &str) -> Option<Class> {
    [Class::Secret, Class::Sensitive].into_iter().find(|c| {
        let p = c.prefix();
        text.len() == p.len() + RANDOM_LEN
            && text.starts_with(p)
            && text[p.len()..].bytes().all(|b| BASE32.contains(&b))
    })
}

/// Is this text a marker of the form above?
#[must_use]
pub fn is_marker(text: &str) -> bool {
    class_of(text).is_some()
}

/// A marker and the field it was planted into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Canary {
    /// The field, as a path such as `vault.Entry.value`.
    pub path: String,
    /// The marker.
    pub marker: String,
    /// What the marker stands for.
    pub class: Class,
}

/// The markers of one run, by field.
#[derive(Clone, Debug, Default)]
pub struct CanaryTable {
    entries: Vec<Canary>,
}

impl CanaryTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Plant a fresh marker into the field `path` (the same field twice gets the same marker) and give it back.
    pub fn plant(&mut self, path: &str) -> String {
        self.plant_as(path, Class::Secret)
    }

    /// The same for a class (the first class given for a field stays).
    pub fn plant_as(&mut self, path: &str, class: Class) -> String {
        if let Some(c) = self.entries.iter().find(|c| c.path == path) {
            return c.marker.clone();
        }
        let marker = fresh_marker_of(class);
        self.entries.push(Canary {
            path: path.to_owned(),
            marker: marker.clone(),
            class,
        });
        marker
    }

    /// The canaries of one class.
    #[must_use]
    pub fn of_class(&self, class: Class) -> Vec<Canary> {
        self.entries
            .iter()
            .filter(|c| c.class == class)
            .cloned()
            .collect()
    }

    /// A table of canaries made elsewhere (for instance handed to a child process).
    #[must_use]
    pub fn from_canaries(entries: Vec<Canary>) -> Self {
        Self { entries }
    }

    /// The marker of a field, if one was planted.
    #[must_use]
    pub fn marker(&self, path: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|c| c.path == path)
            .map(|c| c.marker.as_str())
    }

    /// Every canary, in the order they were planted.
    #[must_use]
    pub fn all(&self) -> &[Canary] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::{CanaryTable, PREFIX, fresh_marker, is_marker};

    #[test]
    fn a_marker_has_the_form_and_is_fresh() {
        let all: Vec<String> = (0..2000).map(|_| fresh_marker()).collect();
        for m in &all {
            assert!(is_marker(m), "{m}");
            assert!(m.starts_with(PREFIX));
        }
        let mut sorted = all.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), all.len(), "two markers are never equal");
        // the random part uses the whole alphabet over many markers
        let chars: std::collections::BTreeSet<char> =
            all.iter().flat_map(|m| m[PREFIX.len()..].chars()).collect();
        assert_eq!(chars.len(), 32);
    }

    #[test]
    fn the_form_is_checked_strictly() {
        let m = fresh_marker();
        assert!(!is_marker(&m[..m.len() - 1]));
        assert!(!is_marker(&format!("{m}A")));
        assert!(!is_marker(&m.replacen("FSCANARY", "FSCANARX", 1)));
        assert!(
            !is_marker(&format!("{}1", &m[..m.len() - 1])),
            "1 is no base32 character"
        );
        assert!(!is_marker(""));
    }

    #[test]
    fn a_field_has_one_marker_per_run_and_a_table_names_the_field() {
        let mut t = CanaryTable::new();
        let a = t.plant("a.b");
        let b = t.plant("a.c");
        assert_ne!(a, b);
        assert_eq!(t.plant("a.b"), a);
        assert_eq!(t.marker("a.c"), Some(b.as_str()));
        assert_eq!(t.marker("nothing"), None);
        assert_eq!(t.all().len(), 2);
    }
}
