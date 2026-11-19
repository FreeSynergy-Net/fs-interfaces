//! PROPOSAL, not a change: would a narrower comment-word screen lose a real protection? Before and after over EVERY
//! comment of the real contracts (interfaces, methods, parameters, results, structs, fields). `src/secrets.rs` is NOT
//! changed; this test only shows what the rules below would let through, and every comment that would no longer be a hit is
//! listed with its reason (`LOST`). A new comment that changes the set fails here, so the list is read again.
//!
//! The rules (each keeps the broad word list as the safety net; nothing is dropped by default):
//!  - R1 label key: `key` right after `translation`, `label`, `sort`, `primary`, `foreign` or `cache` is a name of a key
//!    in the data sense, not key material;
//!  - R2 negation: a hit word right behind `no` / `not` (a single `a`, `an`, `any` or `the` in between) says there is none.
//!
//! A comment with a hit word that no rule covers, or a hit only through the run-together search of `secrets.rs`
//! (`accesstoken`), stays a hit.

mod support;

use std::collections::BTreeMap;

use fs_mcp::doc_looks_secret;
use support::real::contracts;

/// The comment words of `secrets.rs` (copied: the list is private there).
const DOC_WORDS: &[&str] = &[
    "secret",
    "secrets",
    "password",
    "passwords",
    "passwd",
    "passphrase",
    "passcode",
    "pwd",
    "credential",
    "credentials",
    "token",
    "tokens",
    "mnemonic",
    "key",
    "keys",
    "private",
    "pem",
    "pkcs",
    "jwt",
    "ssh",
    "apikey",
];

fn words(doc: &str) -> Vec<String> {
    doc.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Would the narrower screen still call this comment a hit?
fn narrow_hit(doc: &str) -> bool {
    if !doc_looks_secret(doc) {
        return false;
    }
    let w = words(doc);
    let hits: Vec<usize> = (0..w.len())
        .filter(|i| DOC_WORDS.contains(&w[*i].as_str()))
        .collect();
    if hits.is_empty() {
        return true; // only the run-together search found it: stays
    }
    !hits.iter().all(|&i| {
        let before = |k: usize| i.checked_sub(k).map(|j| w[j].as_str());
        let label_key = w[i].starts_with("key")
            && matches!(
                before(1),
                Some("translation" | "label" | "sort" | "primary" | "foreign" | "cache")
            );
        let negated = matches!(before(1), Some("no" | "not"))
            || (matches!(before(1), Some("a" | "an" | "any" | "the"))
                && matches!(before(2), Some("no" | "not")));
        label_key || negated
    })
}

/// Every comment of the real contracts, by place.
fn comments() -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for c in contracts() {
        let mut put = |place: String, doc: &str| {
            if !doc.trim().is_empty() {
                m.insert(place, doc.to_owned());
            }
        };
        for i in &c.interfaces {
            put(format!("{}/{}", c.file, i.name), &i.doc);
            for me in &i.methods {
                put(format!("{}/{}.{}", c.file, i.name, me.name), &me.doc);
                for p in me.params.iter().chain(&me.results) {
                    put(
                        format!("{}/{}.{}.{}", c.file, i.name, me.name, p.name),
                        &p.doc,
                    );
                }
            }
        }
        for s in &c.structs {
            put(format!("{}/{}", c.file, s.name), &s.doc);
            for f in &s.fields {
                put(format!("{}/{}.{}", c.file, s.name, f.name), &f.doc);
            }
        }
    }
    m
}

/// The comments that are a hit today and would not be with the narrower rules, each with the reason it is safe.
const LOST: &[(&str, &str)] = &[
    (
        "common/SupportReport.entries",
        "R2: the only hit word is `secret` in \"The id is no secret\"; the comment is about the field `instance` (it hangs on `entries` by mistake, see CHANGELOG), and it says the opposite of a secret",
    ),
    (
        "people/Persons.patch",
        "R1: the only hit word is `key` in \"translation key suffix\"; a label key of the language files, no key material",
    ),
    (
        "streaming/Target.note",
        "R2: \"with no key in it\" says the field holds no key; it is a note about a problem, and the field is a Text the screen judges by its name (`note`) as well",
    ),
    (
        "types/CommandDescription.label",
        "R1: \"translation key\" only; a label key of the language files",
    ),
    (
        "types/FieldDescription.label",
        "R1: \"translation key (coding.md section 2a), never a sentence\"; a label key of the language files",
    ),
    (
        "types/TypeDescription.label",
        "R1: \"translation key of the type\"; a label key of the language files",
    ),
];

#[test]
fn before_and_after_over_every_real_comment() {
    let all = comments();
    let before: Vec<&String> = all
        .iter()
        .filter(|(_, d)| doc_looks_secret(d))
        .map(|(p, _)| p)
        .collect();
    let after: Vec<&String> = all
        .iter()
        .filter(|(_, d)| narrow_hit(d))
        .map(|(p, _)| p)
        .collect();
    assert!(
        after.iter().all(|p| before.contains(p)),
        "the narrow screen never adds a hit"
    );
    let lost: Vec<(&str, String)> = before
        .iter()
        .filter(|p| !after.contains(p))
        .map(|p| (p.as_str(), all[*p].replace('\n', " ")))
        .collect();
    println!(
        "comments {} before {} after {} lost {}",
        all.len(),
        before.len(),
        after.len(),
        lost.len()
    );
    for (p, d) in &lost {
        println!("LOST {p} :: {d}");
    }
    let listed: Vec<&str> = LOST.iter().map(|(p, _)| *p).collect();
    let got: Vec<&str> = lost.iter().map(|(p, _)| *p).collect();
    assert_eq!(
        got, listed,
        "every lost comment is listed below with its reason"
    );
    for (p, why) in LOST {
        assert!(why.len() > 20, "{p}: a reason is required");
    }
}
