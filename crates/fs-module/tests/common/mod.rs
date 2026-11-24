//! The fixture and the runners shared by the suite tests (the fake in this process).
#![allow(dead_code)]

use fs_interfaces::formsource_capnp::form_source;
use fs_module::conformance::run_conformance;
use fs_module::{
    BlobSample, DeclaredCommand, DeclaredForm, FakeSource, FormSourceFixture, LaneCase, Mode,
    Ordered, Sample, ValueField, ValueKind,
};

/// The client, whether the fake has an image, how many boards it has, the level it says and a second connection to the same module.
pub struct Fixture(
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
    fn value_fields(&self) -> Vec<ValueField> {
        [
            ("description", ValueKind::Description),
            ("color", ValueKind::Color),
            ("estimated", ValueKind::Estimated),
            ("column-limit", ValueKind::ColumnLimit),
            ("priority", ValueKind::Priority),
            ("state", ValueKind::State),
            ("time-zone", ValueKind::TimeZone),
            ("assignee", ValueKind::Assignee),
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

pub async fn run(mode: Mode, blob: bool, boards: usize) -> fs_module::conformance::Report {
    run_level(mode, blob, boards, 2).await
}

/// The fake says `says` in `hello`; the fixture says `claims` (what the suite believes of the service).
pub async fn run_with(mode: Mode, says: u32, claims: u32) -> fs_module::conformance::Report {
    let fake = FakeSource::new(mode).with_level(says);
    let two = capnp_rpc::new_client(fake.connection());
    let fx = Fixture(capnp_rpc::new_client(fake), false, 1, claims, Some(two));
    run_conformance(&fx).await
}

pub async fn run_level(
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
