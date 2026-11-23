//! The conformance rules against the fake (empty and on the sample), the sample's own checks, and mutants that
//! break one rule each (the rules must notice).

mod fx;
mod wrap;

use fs_group_directory::FakeGroups;
use fs_group_directory::conformance::run_conformance;
use fx::Fx;
use std::sync::Arc;
use wrap::{Mutant, Wrap};

/// A floor for the number of checks: a rule whose setup fails quietly would lower it.
const FLOOR: usize = 500;

#[tokio::test]
async fn the_fake_is_conformant_on_an_empty_provider() {
    let report = run_conformance(&Fx::new(FakeGroups::new())).await;
    assert!(
        report.failures.is_empty(),
        "failed rules: {:#?}",
        report.failures
    );
    assert!(
        report.skipped.is_empty(),
        "skipped rules: {:#?}",
        report.skipped
    );
    assert!(
        report.checked >= FLOOR,
        "only {} checks ran",
        report.checked
    );
}

#[tokio::test]
async fn the_fake_is_conformant_on_the_sample() {
    let sample = fs_group_directory::sample();
    assert!(
        sample.problems().is_empty(),
        "the sample build failed: {:#?}",
        sample.problems()
    );
    let report = run_conformance(&Fx::new(sample.groups)).await;
    assert!(
        report.failures.is_empty(),
        "failed rules: {:#?}",
        report.failures
    );
    assert!(
        report.checked >= FLOOR,
        "only {} checks ran",
        report.checked
    );
}

/// Run the rules against a fake with one broken rule; the failures must name the rule.
async fn broken(mutant: Mutant) -> Vec<String> {
    let fake = Arc::new(FakeGroups::new());
    let fx = Fx::with(Arc::clone(&fake), Box::new(Wrap(fake, mutant)));
    run_conformance(&fx).await.failures
}

#[tokio::test]
async fn a_provider_that_leaks_a_hidden_group_through_members_fails() {
    let failures = broken(Mutant::MembersLeak).await;
    assert!(
        failures.iter().any(|f| f.contains("members is NotFound")),
        "{failures:#?}"
    );
}

#[tokio::test]
async fn a_provider_that_answers_forbidden_in_nesting_effect_for_a_hidden_group_fails() {
    let failures = broken(Mutant::EffectLeak).await;
    assert!(
        failures.iter().any(|f| f.contains("nesting_effect")),
        "{failures:#?}"
    );
}

#[tokio::test]
async fn a_provider_that_calls_a_cycle_already_there_fails() {
    let failures = broken(Mutant::CycleAsAlready).await;
    assert!(
        failures.iter().any(|f| f.contains("cycle")),
        "{failures:#?}"
    );
}

#[tokio::test]
async fn a_provider_without_compare_and_set_fails() {
    let failures = broken(Mutant::NoCompareAndSet).await;
    assert!(
        failures
            .iter()
            .any(|f| f.contains("old revision is Changed")),
        "{failures:#?}"
    );
}
