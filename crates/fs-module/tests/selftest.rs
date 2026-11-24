//! The suite against the fake in this process, and the counter-proof (rule 13): a fake that breaks a rule must turn the suite red.

use fs_interfaces::forms::{FORM_SOURCE_LEVEL, FORM_SOURCE_MIN_LEVEL};
use fs_interfaces::formsource_capnp::form_source;
use fs_module::conformance::run_conformance;
use fs_module::{
    BlobSample, DeclaredCommand, DeclaredForm, FakeSource, FormSourceFixture, LaneCase, Mode,
    Ordered, Sample,
};
use tokio::task::LocalSet;

/// The client, whether the fake has an image, how many boards it has, the level it says and a second connection to the same module.
struct Fixture(
    form_source::Client,
    bool,
    usize,
    u32,
    Option<form_source::Client>,
);

impl FormSourceFixture for Fixture {
    fn client(&self) -> form_source::Client {
        self.0.clone()
    }
    fn level(&self) -> u32 {
        self.3
    }
    fn second(&self) -> Option<form_source::Client> {
        self.4.clone()
    }
    fn provider(&self) -> Option<&str> {
        Some("fake")
    }
    fn forms(&self) -> Vec<DeclaredForm> {
        FakeSource::forms()
    }
    fn commands(&self) -> Vec<DeclaredCommand> {
        FakeSource::commands()
    }
    fn samples(&self) -> Vec<Sample> {
        FakeSource::samples()
    }
    fn strict_scope(&self) -> bool {
        true
    }
    fn forbidden(&self) -> Option<(String, String)> {
        Some(("card.move".into(), "c3".into()))
    }
    fn forbidden_set(&self) -> Option<String> {
        Some("card.clear".into())
    }
    fn blob_sample(&self) -> Option<BlobSample> {
        self.1.then(|| BlobSample {
            record: "c1".into(),
            field: "image".into(),
        })
    }
    fn time_fields(&self) -> Vec<(String, String)> {
        vec![("cards-in-columns@1.0.0".into(), "due".into())]
    }
    fn ordered(&self) -> Option<Ordered> {
        Some(Ordered {
            command: "card.move".into(),
            form: "cards-in-columns@1.0.0".into(),
            record: "c1".into(),
            neighbour: "c3".into(),
            args: vec![("column".into(), "done".into())],
        })
    }
    fn create_without_lane(&self) -> Option<LaneCase> {
        Some(LaneCase {
            command: "card.create".into(),
            args: vec![
                ("title".into(), "No lane".into()),
                ("column".into(), "todo".into()),
            ],
            accepted: self.2 == 1,
        })
    }
}

async fn run(mode: Mode, blob: bool, boards: usize) -> fs_module::conformance::Report {
    run_level(mode, blob, boards, 2).await
}

/// The fake says `says` in `hello`; the fixture says `claims` (what the suite believes of the service).
async fn run_with(mode: Mode, says: u32, claims: u32) -> fs_module::conformance::Report {
    let fake = FakeSource::new(mode).with_level(says);
    let two = capnp_rpc::new_client(fake.connection());
    let fx = Fixture(capnp_rpc::new_client(fake), false, 1, claims, Some(two));
    run_conformance(&fx).await
}

async fn run_level(
    mode: Mode,
    blob: bool,
    boards: usize,
    level: u32,
) -> fs_module::conformance::Report {
    let mut fake = FakeSource::new(mode).with_boards(boards).with_level(level);
    if blob {
        fake = fake.with_blob();
    }
    let two = capnp_rpc::new_client(fake.connection());
    run_conformance(&Fixture(
        capnp_rpc::new_client(fake),
        blob,
        boards,
        level,
        Some(two),
    ))
    .await
}

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

/// Counter-proof of rule 1: a level below the minimum or above the newest one is red.
#[tokio::test(flavor = "current_thread")]
async fn a_level_outside_min_to_level_turns_rule_1_red() {
    for says in [FORM_SOURCE_MIN_LEVEL - 1, FORM_SOURCE_LEVEL + 1] {
        let r = LocalSet::new()
            .run_until(run_with(Mode::Strict, says, 1))
            .await;
        assert!(
            r.failures.iter().any(|f| f.starts_with("1 hello: level")),
            "{says}: {:#?}",
            r.failures
        );
    }
    let ok = LocalSet::new()
        .run_until(run_with(Mode::Strict, FORM_SOURCE_MIN_LEVEL, 1))
        .await;
    assert!(
        ok.failures.iter().all(|f| !f.starts_with("1 hello")),
        "{:#?}",
        ok.failures
    );
}
