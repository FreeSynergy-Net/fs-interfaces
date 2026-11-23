//! The fake passes every conformance rule; every mutant (a fake with one switched-on defect) breaks the rules named by
//! `rules_for`; a provider with short reads passes everything; a read-only tree that accepts a change is caught.

use fs_storage::conformance::{Fixture, Report, run};
use fs_storage::{ALL_FAULTS, FakeLocal, FakeStorage, Fault, rules_for, sample};

fn fixture(fault: Option<Fault>, local: FakeLocal, limit: Option<u64>) -> Fixture {
    let s = sample();
    let mut base = FakeStorage::new(&s.anna);
    if let Some(f) = fault {
        base = base.with_fault(f);
    }
    if let Some(l) = limit {
        base = base.with_read_limit(l);
    }
    Fixture {
        ben: Box::new(base.as_person(&s.ben)),
        fremd: Box::new(base.as_person(&s.fremd)),
        admin: Box::new(base.as_person(&s.admin)),
        operator: Box::new(base.as_person(&s.operator)),
        bot: Box::new(base.as_person(&s.bot)),
        anna: Box::new(base),
        local: Box::new(local),
        bot_ref: s.bot,
    }
}

/// A fresh world per rule: the closure builds a new fake every time.
async fn report(fault: Option<Fault>, local: FakeLocal, limit: Option<u64>) -> Report {
    run(&|| fixture(fault, local.clone(), limit)).await
}

#[tokio::test]
async fn the_fake_passes_every_rule() {
    let r = report(None, FakeLocal::new(), None).await;
    assert!(r.failed.is_empty(), "failed: {:?}", r.failed);
    assert!(
        r.passed.len() > 150,
        "the matrix rows ran: {}",
        r.passed.len()
    );
}

#[tokio::test]
async fn every_mutant_breaks_a_rule_named_for_it() {
    for &fault in ALL_FAULTS {
        let r = report(Some(fault), FakeLocal::new(), None).await;
        let wanted = rules_for(fault);
        let hit = r.failed.iter().any(|(rule, _)| {
            wanted
                .iter()
                .any(|w| rule == w || rule.starts_with(&format!("{w}:")))
        });
        assert!(
            hit,
            "{fault:?} survived {wanted:?}; failed: {:?}",
            r.failed.iter().map(|f| &f.0).collect::<Vec<_>>()
        );
    }
}

#[tokio::test]
async fn a_read_only_tree_that_changes_is_caught() {
    let mutants = [
        FakeLocal::new().with_write_bug(),
        FakeLocal::new().with_accepts("create"),
        FakeLocal::new().with_accepts("rename"),
        FakeLocal::new().with_accepts("move_to"),
        FakeLocal::new().with_accepts("copy_to"),
        FakeLocal::new().with_accepts("delete"),
        FakeLocal::new().with_accepts("restore"),
    ];
    for m in mutants {
        let r = report(None, m.clone(), None).await;
        assert!(
            r.failed
                .iter()
                .any(|(rule, _)| rule.contains("T13") || rule == "local.import-only"),
            "{m:?} survived: {:?}",
            r.failed
        );
    }
}

#[tokio::test]
async fn a_provider_with_short_reads_passes_every_rule() {
    let r = report(None, FakeLocal::new(), Some(3)).await;
    assert!(r.failed.is_empty(), "failed: {:?}", r.failed);
}

/// Every fault a row names must break THAT row (not just some other rule). The table's column is a claim; this proves it.
#[tokio::test]
async fn every_named_fault_breaks_its_row() {
    let rows = fs_storage::conformance::matrix::rows();
    let mut false_claims = Vec::new();
    for &fault in ALL_FAULTS {
        let r = report(Some(fault), FakeLocal::new(), None).await;
        for row in &rows {
            let broken = r.failed_rule(&row.rule());
            if row.faults.contains(&fault) && !broken {
                false_claims.push(format!("{fault:?} does not break {}", row.rule()));
            }
        }
    }
    assert!(false_claims.is_empty(), "false claims: {false_claims:#?}");
}
