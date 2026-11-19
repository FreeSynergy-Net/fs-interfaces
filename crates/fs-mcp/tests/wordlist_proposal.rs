//! The comment-word rules, before and after, over EVERY comment of the real contracts (interfaces, methods, parameters,
//! results, structs, fields).
//!
//!  - R1 label key (ACCEPTED at the review of M4, in `src/secrets.rs`): `key`/`keys` right behind `translation`, `label`, `sort`,
//!    `primary`, `foreign` or `cache` is a key in the data sense, not key material. `R1_LOST` lists every comment that was a
//!    hit before and is none now, each with its reason; a new comment that changes the set fails here.
//!  - R2 negation (PROPOSAL, REJECTED at the review: a comment can be wrong, a single case gets a written release per place):
//!    a hit word right behind `no`/`not` says there is none. `R2_WOULD_LOSE` shows what it would have dropped; `secrets.rs`
//!    does not do it.
//!
//! A hit only through the run-together search of `secrets.rs` (`accesstoken`) stays a hit under both.

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

/// Would the screen with rule R2 on top still call this comment a hit?
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
        matches!(before(1), Some("no" | "not"))
            || (matches!(before(1), Some("a" | "an" | "any" | "the"))
                && matches!(before(2), Some("no" | "not")))
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

/// Comments that were a hit before R1 and are none now (R1 is in `secrets.rs`), each with its reason.
const R1_LOST: &[(&str, &str)] = &[
    (
        "people/Persons.patch",
        "the only hit word is `key` in \"translation key suffix\"; a label key of the language files, no key material",
    ),
    (
        "types/CommandDescription.label",
        "\"translation key\" only; a label key of the language files",
    ),
    (
        "types/FieldDescription.label",
        "\"translation key (coding.md section 2a), never a sentence\"; a label key of the language files",
    ),
    (
        "types/TypeDescription.label",
        "\"translation key of the type\"; a label key of the language files",
    ),
];

/// What rule R2 would drop on top (REJECTED, not in `secrets.rs`).
const R2_WOULD_LOSE: &[(&str, &str)] = &[
    (
        "common/SupportReport.entries",
        "the only hit word is `secret` in \"The id is no secret\"; the comment belongs to `instance` (it hangs on `entries` by mistake), and it says the opposite of a secret",
    ),
    (
        "streaming/Target.note",
        "\"with no key in it\" says the field holds no key",
    ),
];

/// Was the comment a hit by the plain word list, before R1 (no exemption for a label key)?
fn hit_before_r1(doc: &str) -> bool {
    doc_looks_secret(doc) || words(doc).iter().any(|w| DOC_WORDS.contains(&w.as_str()))
}

#[test]
fn rule_r1_loses_exactly_the_listed_label_comments() {
    let all = comments();
    let lost: Vec<&str> = all
        .iter()
        .filter(|(_, d)| hit_before_r1(d) && !doc_looks_secret(d))
        .map(|(p, _)| p.as_str())
        .collect();
    let listed: Vec<&str> = R1_LOST.iter().map(|(p, _)| *p).collect();
    assert_eq!(
        lost, listed,
        "every comment that R1 lets through is listed with its reason"
    );
    for (p, why) in R1_LOST {
        assert!(why.len() > 20, "{p}: a reason is required");
        assert!(
            words(&all[*p]).iter().any(|w| w.starts_with("key")),
            "{p}: R1 is about `key`"
        );
    }
    // `key` behind another word is still a hit
    for doc in [
        "translation secret",
        "translation private",
        "label jwt",
        "label password",
        "sort token",
        "label key and the password",
        "the allowed keys",
        "an api key",
        "the key of the vault",
        "translation of the key",
        "keys",
    ] {
        assert!(doc_looks_secret(doc), "{doc}");
    }
    for doc in [
        "translation key",
        "a label key",
        "the sort key",
        "primary key",
        "foreign keys",
        "cache key of the list",
    ] {
        assert!(!doc_looks_secret(doc), "{doc}");
    }
}

#[test]
fn rule_r2_is_not_in_the_screen_and_would_drop_only_these() {
    let all = comments();
    let would: Vec<&str> = all
        .iter()
        .filter(|(_, d)| doc_looks_secret(d) && !narrow_hit(d))
        .map(|(p, _)| p.as_str())
        .collect();
    let listed: Vec<&str> = R2_WOULD_LOSE.iter().map(|(p, _)| *p).collect();
    assert_eq!(would, listed);
    // and they are hits today: the screen keeps them
    for (p, _) in R2_WOULD_LOSE {
        assert!(doc_looks_secret(&all[*p]), "{p}");
    }
}
