//! The fake passes every conformance rule; every mutant (a fake with one switched-on defect) is caught by its rule.

use fs_storage::conformance::{Fixture, Report, run};
use fs_storage::{FakeLocal, FakeStorage, Fault, sample};

fn fixture(fault: Option<Fault>, local: FakeLocal) -> Fixture {
    let s = sample();
    let base = FakeStorage::new(&s.anna);
    let base = match fault {
        Some(f) => base.with_fault(f),
        None => base,
    };
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
async fn report(fault: Option<Fault>, local: FakeLocal) -> Report {
    run(&|| fixture(fault, local.clone())).await
}

#[tokio::test]
async fn the_fake_passes_every_rule() {
    let r = report(None, FakeLocal::new()).await;
    assert!(r.failed.is_empty(), "failed: {:?}", r.failed);
    assert_eq!(r.passed.len(), 27, "all rules ran: {:?}", r.passed);
}

#[tokio::test]
async fn every_mutant_is_caught_by_its_rule() {
    let table = [
        (Fault::IgnoreRights, "rights.stranger.read"),
        (Fault::IgnoreRights, "rights.stranger.write"),
        (Fault::IgnoreRights, "rights.member.readonly"),
        (Fault::IgnoreRights, "rights.bot"),
        (Fault::SkipNameCheck, "names.bad"),
        (Fault::FollowLinks, "link.never-followed"),
        (Fault::WrongSha, "file.sha256"),
        (Fault::Replace, "conflict.never-replace"),
        (Fault::PurgeOnTrash, "trash.restore-purge"),
        (Fault::TrashLeak, "rights.trash.private"),
        (Fault::NoCancel, "write.cancel"),
        (Fault::BotAllowed, "rights.bot"),
        (Fault::AdminReadsAll, "rights.manager.no-server"),
        (Fault::OperatorBlocked, "rights.operator.reads-all"),
        (Fault::OperatorChanges, "rights.operator.reads-all"),
        (Fault::ManagerGroupRoot, "rights.manager.no-server"),
        (Fault::OperatorRoot, "rights.operator.reads-all"),
        (Fault::OperatorLink, "rights.operator.reads-all"),
        (Fault::NoReadCap, "blob.range"),
        (Fault::ManagerChanges, "rights.manager.no-server"),
        (Fault::ZeroRead, "blob.range"),
        (Fault::ReadsTooMuch, "blob.range"),
        (Fault::PathTraversal, "path.traversal"),
        (Fault::BadRange, "blob.range"),
        (Fault::ScopeLeak, "scope.generator"),
    ];
    for (fault, rule) in table {
        let r = report(Some(fault), FakeLocal::new()).await;
        assert!(
            r.failed_rule(rule),
            "{fault:?} survived rule {rule}; failed: {:?}",
            r.failed
        );
    }
}

#[tokio::test]
async fn a_local_tree_that_saves_is_caught() {
    let r = report(None, FakeLocal::new().with_write_bug()).await;
    assert!(r.failed_rule("local.import-only"), "failed: {:?}", r.failed);
}

#[tokio::test]
async fn a_provider_with_short_reads_passes_every_rule() {
    let s = sample();
    let make = || {
        let base = FakeStorage::new(&s.anna).with_read_limit(3);
        Fixture {
            ben: Box::new(base.as_person(&s.ben).with_read_limit(3)),
            fremd: Box::new(base.as_person(&s.fremd).with_read_limit(3)),
            admin: Box::new(base.as_person(&s.admin).with_read_limit(3)),
            operator: Box::new(base.as_person(&s.operator).with_read_limit(3)),
            bot: Box::new(base.as_person(&s.bot).with_read_limit(3)),
            anna: Box::new(base),
            local: Box::new(FakeLocal::new()),
            bot_ref: s.bot.clone(),
        }
    };
    let r = run(&make).await;
    assert!(r.failed.is_empty(), "failed: {:?}", r.failed);
}
