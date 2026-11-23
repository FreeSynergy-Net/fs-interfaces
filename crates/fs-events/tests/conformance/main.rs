//! The conformance rules against the fake (empty and on the sample) and mutants that break one rule each (the rules
//! must notice).
#![allow(clippy::too_many_lines)]

mod fx;
mod mutant;
mod tamper;
mod wrap;

use fs_calendars_testdata::conformance::run_conformance;
use fx::{Fx, new_fake};
use mutant::Mutant;

/// A floor for the number of checks: a rule whose setup fails quietly would lower it.
const FLOOR: usize = 150;

#[tokio::test]
async fn the_fake_is_conformant_on_an_empty_provider() {
    let report = run_conformance(&Fx::plain(&new_fake())).await;
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
    let fake = std::sync::Arc::new(
        fs_calendars_testdata::sample()
            .fake()
            .with_ai(&fs_calendars_testdata::ids::person(8)),
    );
    let report = run_conformance(&Fx::plain(&fake)).await;
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
    run_conformance(&Fx::with(new_fake(), mutant))
        .await
        .failures
}

async fn notices(mutant: Mutant, part: &str) {
    let failures = broken(mutant).await;
    assert!(
        failures.iter().any(|f| f.contains(part)),
        "{mutant:?} was not noticed ({part}): {failures:#?}"
    );
}

#[tokio::test]
async fn a_purge_that_ignores_the_retention_fails() {
    notices(Mutant::PurgeIgnoresHold, "kept appointment is Forbidden").await;
}
#[tokio::test]
async fn a_purge_that_deletes_nothing_fails() {
    notices(Mutant::PurgeDoesNothing, "no trash view any more").await;
}
#[tokio::test]
async fn not_shared_answered_as_unknown_fails() {
    notices(Mutant::NotSharedAsUnknown, "does not share is NotShared").await;
}
#[tokio::test]
async fn an_unknown_participant_guessed_as_free_fails() {
    notices(Mutant::UnknownGuessedFree, "is Unknown").await;
}
#[tokio::test]
async fn writing_into_a_trashed_calendar_as_invalid_fails() {
    notices(Mutant::TrashedWriteInvalid, "is NotFound(calendar)").await;
}
#[tokio::test]
async fn read_only_answered_as_forbidden_fails() {
    notices(Mutant::ReadOnlyAsForbidden, "is ReadOnly").await;
}
#[tokio::test]
async fn a_feed_on_the_cloud_metadata_address_fails() {
    notices(Mutant::FeedAcceptsMetadata, "169.254.169.254").await;
}
#[tokio::test]
async fn a_retention_end_lost_in_never_mode_fails() {
    notices(Mutant::HeldEndLost, "Held(Some(end))").await;
}
#[tokio::test]
async fn an_ai_owner_that_is_invalid_instead_of_forbidden_fails() {
    notices(Mutant::AiOwnerInvalid, "Forbidden(calendar.create)").await;
}
#[tokio::test]
async fn update_calendar_without_compare_and_set_fails() {
    notices(Mutant::NoCompareAndSet, "stale revision is Changed").await;
}
#[tokio::test]
async fn a_hidden_calendar_that_is_forbidden_for_an_ai_fails() {
    notices(Mutant::HiddenForbidden, "NotFound for an AI").await;
}
#[tokio::test]
async fn the_feed_address_in_the_origin_fails() {
    notices(Mutant::FeedInOrigin, "never the feed address").await;
}
#[tokio::test]
async fn delete_calendar_that_loses_the_count_fails() {
    notices(Mutant::CountLost, "and says how many").await;
}
#[tokio::test]
async fn a_feed_with_a_foreign_scheme_fails() {
    notices(Mutant::FeedAccepts("ftp://"), "ftp://kalender").await;
}
#[tokio::test]
async fn a_feed_with_credentials_fails() {
    notices(Mutant::FeedAccepts("secret@"), "user:secret@").await;
}
#[tokio::test]
async fn a_feed_on_an_embedded_v4_in_v6_fails() {
    notices(Mutant::FeedAccepts("::ffff:"), "::ffff:10.0.0.1").await;
}
#[tokio::test]
async fn a_feed_on_a_nat64_address_fails() {
    notices(Mutant::FeedAccepts("64:ff9b"), "64:ff9b").await;
}
#[tokio::test]
async fn a_feed_on_a_6to4_address_fails() {
    notices(Mutant::FeedAccepts("2002:"), "2002:a00").await;
}
#[tokio::test]
async fn a_feed_on_a_file_scheme_fails() {
    notices(Mutant::FeedAccepts("file://"), "file:///etc/passwd").await;
}
#[tokio::test]
async fn a_purge_of_a_living_object_that_is_ok_fails() {
    notices(Mutant::PurgeLivingOk, "a living appointment is Invalid").await;
}
#[tokio::test]
async fn a_retention_without_end_shown_as_manual_fails() {
    notices(Mutant::HeldNoneAsManual, "Held(None)").await;
}
#[tokio::test]
async fn a_calendar_restore_without_cascade_fails() {
    notices(
        Mutant::RestoreNoCascade,
        "the appointments that went with it are back",
    )
    .await;
}
#[tokio::test]
async fn the_address_shown_to_a_stranger_fails() {
    notices(Mutant::AddressShown, "does not see the address").await;
}
#[tokio::test]
async fn an_ai_delete_that_is_invalid_instead_of_forbidden_fails() {
    notices(Mutant::AiDeleteInvalid, "Forbidden(calendar.delete)").await;
}
#[tokio::test]
async fn an_ai_subscribe_that_is_invalid_instead_of_forbidden_fails() {
    notices(Mutant::AiSubscribeInvalid, "Forbidden(calendar.subscribe)").await;
}
#[tokio::test]
async fn an_ai_purge_that_is_invalid_instead_of_forbidden_fails() {
    notices(Mutant::AiPurgeInvalid, "Forbidden(trash.purge)").await;
}
#[tokio::test]
async fn restoring_in_a_subscription_as_forbidden_fails() {
    notices(
        Mutant::RestoreSubscribedForbidden,
        "restore in a subscription is ReadOnly",
    )
    .await;
}
#[tokio::test]
async fn a_deadline_that_takes_a_kept_calendar_fails() {
    notices(
        Mutant::DeadlineIgnoresHold,
        "holds its trashed calendar beyond the period",
    )
    .await;
}
#[tokio::test]
async fn an_original_rule_that_can_be_made_fails() {
    notices(Mutant::OriginalAccepted, "cannot be made").await;
}
#[tokio::test]
async fn a_kept_calendar_that_loses_its_appointments_fails() {
    notices(
        Mutant::HeldCalendarLosesAppointments,
        "a kept calendar keeps its appointments",
    )
    .await;
}
#[tokio::test]
async fn a_held_calendar_that_cannot_be_restored_fails() {
    notices(
        Mutant::HeldCalendarNotRestorable,
        "a calendar shown as Held can be restored",
    )
    .await;
}
