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

// ---- groups

mod operator;
mod operator_unit;
mod wrap_governance;
use fs_group_directory::conformance::run_conformance_governance;
use wrap_governance::{GovMutant, WrapGov};

/// A floor for the gov checks.
const FLOOR_GOVERNANCE: usize = 130;

/// A fake whose asking person is the installer (she hands out the first `Project` scope), and the gov view of it.
fn fx_gov(mutant: GovMutant) -> Fx {
    let people = Arc::new(fs_group_directory::FakePeople::new());
    let me = people.human("Alex Installer");
    let fake = Arc::new(FakeGroups::with_people(people, me.clone()).with_installer(me));
    let gov = Box::new(WrapGov(Arc::clone(&fake), mutant));
    Fx::with(Arc::clone(&fake), Box::new(Wrap(fake, Mutant::None))).with_governance(gov)
}

async fn broken_gov(mutant: GovMutant) -> Vec<String> {
    run_conformance_governance(&fx_gov(mutant)).await.failures
}

#[tokio::test]
async fn the_fake_is_conformant_to_groups() {
    let report = run_conformance_governance(&fx_gov(GovMutant::None)).await;
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
        report.checked >= FLOOR_GOVERNANCE,
        "only {} checks ran",
        report.checked
    );
}

#[tokio::test]
async fn the_fake_still_passes_the_base_rules_after_governance() {
    let report = run_conformance(&fx_gov(GovMutant::None)).await;
    assert!(
        report.failures.is_empty(),
        "failed rules: {:#?}",
        report.failures
    );
}

#[tokio::test]
async fn governance_without_a_view_is_skipped_not_passed() {
    let report = run_conformance_governance(&Fx::new(FakeGroups::new())).await;
    assert!(report.failures.is_empty() && !report.skipped.is_empty());
}

#[tokio::test]
async fn a_provider_that_gives_an_empty_list_for_a_withheld_group_fails() {
    let f = broken_gov(GovMutant::EmptyForWithheld).await;
    assert!(f.iter().any(|x| x.contains("Withheld")), "{f:#?}");
}

#[tokio::test]
async fn a_provider_that_lets_anyone_attach_an_addon_fails() {
    let f = broken_gov(GovMutant::AddonWithoutRight).await;
    assert!(f.iter().any(|x| x.contains("attach_addon")), "{f:#?}");
}

#[tokio::test]
async fn a_provider_that_lets_the_last_owner_go_fails() {
    let f = broken_gov(GovMutant::LastOwnerGoes).await;
    assert!(f.iter().any(|x| x.contains("LastOwner")), "{f:#?}");
}

#[tokio::test]
async fn a_provider_that_takes_the_actor_from_an_argument_fails() {
    let f = broken_gov(GovMutant::ActorFromArgument).await;
    assert!(
        f.iter().any(|x| x.contains("the actor is the connection")),
        "{f:#?}"
    );
}

#[tokio::test]
async fn a_provider_where_a_manager_appoints_owners_fails() {
    let f = broken_gov(GovMutant::AppointWithoutOwner).await;
    assert!(f.iter().any(|x| x.contains("appoint_owner")), "{f:#?}");
}

#[tokio::test]
async fn a_provider_that_takes_an_intervention_without_a_reason_fails() {
    let f = broken_gov(GovMutant::NoReason).await;
    assert!(f.iter().any(|x| x.contains("reason")), "{f:#?}");
}

#[tokio::test]
async fn an_addon_that_changes_members_fails() {
    let f = broken_gov(GovMutant::AddonChangesMembers).await;
    assert!(f.iter().any(|x| x.contains("only attached")), "{f:#?}");
}

// ---- direction: members and rights flow over BottomUp one way, over TopDown the other way, over Single not at all

async fn broken_base(mutant: Mutant) -> Vec<String> {
    let fake = Arc::new(FakeGroups::new());
    run_conformance(&Fx::with(Arc::clone(&fake), Box::new(Wrap(fake, mutant))))
        .await
        .failures
}

#[tokio::test]
async fn a_provider_that_swaps_the_direction_of_an_edge_fails_in_both_trees() {
    let f = broken_base(Mutant::SwapDirection).await;
    assert!(f.iter().any(|x| x.starts_with("bottom-up:")), "{f:#?}");
    assert!(f.iter().any(|x| x.starts_with("top-down:")), "{f:#?}");
    assert!(
        f.iter().any(|x| x.starts_with("mirror trees: top-down")),
        "{f:#?}"
    );
}

#[tokio::test]
async fn a_provider_where_a_single_edge_lets_things_flow_fails() {
    let f = broken_base(Mutant::SingleFlows).await;
    assert!(f.iter().any(|x| x.starts_with("single:")), "{f:#?}");
    assert!(
        f.iter().any(|x| x.contains("single edge is not mirrored")),
        "{f:#?}"
    );
}

#[tokio::test]
async fn a_mirror_that_does_not_reverse_a_top_down_edge_fails() {
    let f = broken_base(Mutant::TopDownNotReversed).await;
    assert!(f.iter().any(|x| x.starts_with("top-down:")), "{f:#?}");
    assert!(
        f.iter().any(|x| x.starts_with("mirror trees: top-down")),
        "{f:#?}"
    );
    assert!(!f.iter().any(|x| x.starts_with("bottom-up:")), "{f:#?}");
}

/// MUST 1 of the review: a founder with a hidden personal `Manage` keeps it after handing over and leaving.
#[tokio::test]
async fn a_provider_where_the_founder_keeps_a_personal_manage_grant_fails() {
    let people = Arc::new(fs_group_directory::FakePeople::new());
    let me = people.human("Alex Installer");
    let fake = Arc::new(FakeGroups::with_people(people, me.clone()).with_installer(me));
    let gov = Box::new(WrapGov(Arc::clone(&fake), GovMutant::None));
    let fx = Fx::with(
        Arc::clone(&fake),
        Box::new(Wrap(fake, Mutant::CreatorKeepsManage)),
    )
    .with_governance(gov);
    let f = run_conformance_governance(&fx).await.failures;
    assert!(
        f.iter().any(|x| x.contains("former owner holds no Manage")),
        "{f:#?}"
    );
}
