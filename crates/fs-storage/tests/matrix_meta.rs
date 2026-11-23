//! The meta test: every method of the traits is in the matrix (a table row or a named rule), the table is complete, and
//! every fault has a rule. A new trait method or fault without an entry turns this red.

use fs_storage::conformance::matrix::{
    COVERED_BY_RULES, Expect, METHODS, Method, Row, Target, rows,
};
use fs_storage::conformance::{Fixture, run};
use fs_storage::{ALL_FAULTS, FakeLocal, FakeStorage, sample};
use std::collections::BTreeSet;

const PROVIDER_SRC: &str = include_str!("../src/provider.rs");
const CTL_SRC: &str = include_str!("../src/ctl.rs");
const FAULT_SRC: &str = include_str!("../src/fake/state.rs");

/// The `fn <name>(` of one trait block (braces counted).
fn trait_fns(src: &str, header: &str) -> BTreeSet<String> {
    let start = src.find(header).unwrap_or_else(|| panic!("no {header}"));
    let body = &src[start..];
    let (mut depth, mut end) = (0_i32, body.len());
    for (i, c) in body.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    let mut out = BTreeSet::new();
    let mut rest = &body[..end];
    while let Some(i) = rest.find("fn ") {
        rest = &rest[i + 3..];
        if let Some(j) = rest.find('(') {
            out.insert(rest[..j].trim().to_owned());
        }
    }
    out
}

#[test]
fn every_trait_method_is_in_the_matrix() {
    let mut src = trait_fns(PROVIDER_SRC, "pub trait StorageProvider");
    src.extend(trait_fns(CTL_SRC, "pub trait Blob"));
    let listed: BTreeSet<String> = METHODS.iter().map(|m| m.name().to_owned()).collect();
    assert_eq!(
        src, listed,
        "the trait source and the matrix method list differ"
    );
}

#[test]
fn every_method_has_rows_or_a_named_rule() {
    let table: BTreeSet<Method> = rows().iter().map(|r| r.op.method()).collect();
    let named: BTreeSet<Method> = COVERED_BY_RULES.iter().map(|(m, _)| *m).collect();
    for m in METHODS {
        assert!(
            table.contains(&m) || named.contains(&m),
            "{m:?} is neither in the table nor covered by a rule"
        );
    }
}

#[tokio::test]
async fn the_named_rules_exist() {
    let s = sample();
    let make = || {
        let base = FakeStorage::new(&s.anna);
        Fixture {
            ben: Box::new(base.as_person(&s.ben)),
            fremd: Box::new(base.as_person(&s.fremd)),
            admin: Box::new(base.as_person(&s.admin)),
            operator: Box::new(base.as_person(&s.operator)),
            bot: Box::new(base.as_person(&s.bot)),
            anna: Box::new(base.clone()),
            local: Box::new(FakeLocal::new()),
            bot_ref: s.bot.clone(),
        }
    };
    let r = run(&make).await;
    for (m, names) in COVERED_BY_RULES {
        for n in *names {
            assert!(
                r.passed.iter().any(|p| p == n),
                "{m:?}: the rule {n} does not exist"
            );
        }
    }
}

fn row_ok(r: &Row) -> bool {
    let cells = r.parsed();
    let same =
        r.target != Target::T13 || cells.windows(2).all(|w| w[0] == w[1] || w[1] == Expect::Na);
    // A row names faults, or says in `note` why it has none (a row with faults may still carry a note).
    same && (!r.faults.is_empty() || r.note.trim().len() > 4)
}

#[test]
fn the_table_is_complete_and_unique() {
    let all = rows();
    let mut seen = BTreeSet::new();
    for r in &all {
        assert!(
            seen.insert((r.op.name(), format!("{:?}", r.target))),
            "duplicate row {r:?}"
        );
        assert!(row_ok(r), "bad row {r:?}");
    }
    assert!(all.len() > 100);
}

#[test]
fn every_fault_is_listed() {
    let start = FAULT_SRC.find("pub enum Fault").expect("enum");
    let block = &FAULT_SRC[start..];
    let block = &block[..block.find("\n}").expect("end")];
    let variants = block
        .lines()
        .filter(|l| {
            l.starts_with("    ")
                && !l.trim_start().starts_with("//")
                && l.trim_end().ends_with(',')
        })
        .count();
    assert_eq!(
        variants,
        ALL_FAULTS.len(),
        "ALL_FAULTS misses a fault or lists one twice"
    );
    let unique: BTreeSet<String> = ALL_FAULTS.iter().map(|f| format!("{f:?}")).collect();
    assert_eq!(unique.len(), ALL_FAULTS.len());
}

#[test]
#[should_panic(expected = "unknown matrix code")]
fn a_provisional_cell_is_refused() {
    let _ = Expect::parse("INV?");
}

#[test]
fn no_cell_of_the_table_is_provisional() {
    for r in rows() {
        assert!(!r.cells.contains('?'), "provisional cell in {r:?}");
    }
}
