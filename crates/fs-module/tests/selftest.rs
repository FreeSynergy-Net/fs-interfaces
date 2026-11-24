//! The suite against the fake in this process, and the counter-proof (rule 13): a fake that breaks a rule must turn the suite red.

use fs_interfaces::formsource_capnp::form_source;
use fs_module::conformance::run_conformance;
use fs_module::{
    BlobSample, DeclaredCommand, DeclaredForm, FakeSource, FormSourceFixture, LaneCase, Mode,
    Ordered, Sample, ValueField, ValueKind,
};
use tokio::task::LocalSet;

/// The client, and whether the fake has an image and how many boards it has.
struct Fixture(form_source::Client, bool, usize);

impl FormSourceFixture for Fixture {
    fn client(&self) -> form_source::Client {
        self.0.clone()
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
    fn value_fields(&self) -> Vec<ValueField> {
        [
            ("description", ValueKind::Description),
            ("color", ValueKind::Color),
            ("estimated", ValueKind::Estimated),
            ("column-limit", ValueKind::ColumnLimit),
            ("priority", ValueKind::Priority),
        ]
        .into_iter()
        .map(|(p, kind)| ValueField {
            form: "cards-in-columns@1.0.0".into(),
            path: p.into(),
            kind,
        })
        .collect()
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
    let mut fake = FakeSource::new(mode).with_boards(boards);
    if blob {
        fake = fake.with_blob();
    }
    run_conformance(&Fixture(capnp_rpc::new_client(fake), blob, boards)).await
}

#[tokio::test(flavor = "current_thread")]
async fn the_strict_fake_passes_every_rule() {
    let r = LocalSet::new().run_until(run(Mode::Strict, true, 1)).await;
    assert!(r.failures.is_empty(), "{:#?}", r.failures);
    // Skipped by design: rule 9 on the locked set command, rule 12 (blob WITHOUT an image field; the next test runs it).
    assert_eq!(r.skipped.len(), 2, "{:#?}", r.skipped);
    assert!(r.checked > 60, "{}", r.checked);
}

#[tokio::test(flavor = "current_thread")]
async fn the_fake_without_image_and_with_two_boards_passes_too() {
    let r = LocalSet::new().run_until(run(Mode::Strict, false, 2)).await;
    assert!(r.failures.is_empty(), "{:#?}", r.failures);
    assert_eq!(r.skipped.len(), 2, "{:#?}", r.skipped); // rule 9 (locked set command), rule 20 (no image field)
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
async fn lax_values_must_turn_rule_22_red_for_each_of_the_five_channels() {
    let r = LocalSet::new()
        .run_until(run(Mode::LaxValues, true, 1))
        .await;
    for path in [
        "description",
        "color",
        "estimated",
        "column-limit",
        "priority",
    ] {
        assert!(
            r.failures
                .iter()
                .any(|f| f.starts_with("22 ") && f.contains(&format!("22 {path} "))),
            "{path}: {:#?}",
            r.failures
        );
    }
}
