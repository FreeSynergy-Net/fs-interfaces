//! The suite against the fake in this process, and the counter-proof (rule 13): a fake that breaks a rule must turn the suite red.

mod common;

use common::{run, run_level, run_with};
use fs_interfaces::forms::{FORM_SOURCE_LEVEL, FORM_SOURCE_MIN_LEVEL};
use fs_module::Mode;
use tokio::task::LocalSet;

#[tokio::test(flavor = "current_thread")]
async fn the_strict_fake_passes_every_rule() {
    let r = LocalSet::new().run_until(run(Mode::Strict, true, 1)).await;
    assert!(r.failures.is_empty(), "{:#?}", r.failures);
    // Skipped by design: rule 9 on the locked set command, rule 12 (blob WITHOUT an image field; the next test runs it), L2-9 gone (no redaction).
    assert_eq!(r.skipped.len(), 3, "{:#?}", r.skipped);
    assert!(r.checked > 60, "{}", r.checked);
}

#[tokio::test(flavor = "current_thread")]
async fn the_fake_without_image_and_with_two_boards_passes_too() {
    let r = LocalSet::new().run_until(run(Mode::Strict, false, 2)).await;
    assert!(r.failures.is_empty(), "{:#?}", r.failures);
    assert_eq!(r.skipped.len(), 3, "{:#?}", r.skipped); // rule 9 (locked set command), rule 20 (no image field), L2-9 gone
}

#[tokio::test(flavor = "current_thread")]
async fn lax_shape_must_turn_rule_14_red() {
    let r = LocalSet::new()
        .run_until(run(Mode::LaxShape, true, 1))
        .await;
    println!("RED: {:#?}", r.failures);
    assert!(
        r.failures.iter().any(|f| f.starts_with("14 ")),
        "{:#?}",
        r.failures
    );
    // A due value with a zone suffix is more than RFC 3339: rule 3 must refuse it.
    assert!(
        r.failures
            .iter()
            .any(|f| f.starts_with("3 ") && f.contains("time field due")),
        "{:#?}",
        r.failures
    );
}

#[tokio::test(flavor = "current_thread")]
async fn lax_errors_must_turn_the_suite_red() {
    let r = LocalSet::new()
        .run_until(run(Mode::LaxErrors, true, 1))
        .await;
    println!("RED: {:#?}", r.failures);
    assert!(
        !r.failures.is_empty(),
        "the suite did not notice a broken error channel"
    );
    assert!(
        r.failures.iter().any(|f| f.starts_with("8 ")),
        "{:#?}",
        r.failures
    );
}

#[tokio::test(flavor = "current_thread")]
async fn a_level_1_service_skips_the_level_2_rules_and_never_passes_them() {
    let r = LocalSet::new()
        .run_until(run_level(Mode::Strict, true, 1, 1))
        .await;
    assert!(r.failures.is_empty(), "{:#?}", r.failures);
    let l2 = r.skipped.iter().filter(|s| s.starts_with("L2-")).count();
    assert_eq!(l2, 10, "{:#?}", r.skipped);
}

/// L2-12: a fake that runs a repeated step twice must turn L2-3 red.
#[tokio::test(flavor = "current_thread")]
async fn ignore_step_must_turn_l2_3_red() {
    let r = LocalSet::new()
        .run_until(run(Mode::IgnoreStep, true, 1))
        .await;
    println!("RED: {:#?}", r.failures);
    assert!(
        r.failures.iter().any(|f| f.starts_with("L2-3 ")),
        "{:#?}",
        r.failures
    );
}

/// Counter-proof of rule 1 (levels are cumulative): a service below `FORM_SOURCE_MIN_LEVEL` is refused with the reason; one above
/// `FORM_SOURCE_LEVEL` runs and is checked only as far as the suite knows (`min(level, LEVEL)`).
#[tokio::test(flavor = "current_thread")]
async fn rule_1_refuses_below_min_and_takes_a_newer_level_using_only_what_it_knows() {
    let low = LocalSet::new()
        .run_until(run_with(
            Mode::Strict,
            FORM_SOURCE_MIN_LEVEL - 1,
            FORM_SOURCE_MIN_LEVEL - 1,
        ))
        .await;
    assert!(
        low.failures
            .iter()
            .any(|f| f.starts_with("1 hello: level is at least FORM_SOURCE_MIN_LEVEL")),
        "{:#?}",
        low.failures
    );
    // Level N+1 against a suite of level N: green, and the level 2 rules ran (none skipped for level).
    let newer = LocalSet::new()
        .run_until(run_with(
            Mode::Strict,
            FORM_SOURCE_LEVEL + 1,
            FORM_SOURCE_LEVEL + 1,
        ))
        .await;
    assert!(newer.failures.is_empty(), "{:#?}", newer.failures);
    assert!(
        newer
            .skipped
            .iter()
            .all(|s| !s.contains("the service is level 1")),
        "{:#?}",
        newer.skipped
    );
    // The fixture and `hello` must agree: a fake that says 2 while the fixture declares 1 is red.
    let liar = LocalSet::new()
        .run_until(run_with(Mode::IgnoreStep, 2, 1))
        .await;
    assert!(
        liar.failures
            .iter()
            .any(|f| f.starts_with("1 hello: level is the level the fixture")),
        "{:#?}",
        liar.failures
    );
    let ok = LocalSet::new()
        .run_until(run_with(
            Mode::Strict,
            FORM_SOURCE_MIN_LEVEL,
            FORM_SOURCE_MIN_LEVEL,
        ))
        .await;
    assert!(ok.failures.is_empty(), "{:#?}", ok.failures);
}
