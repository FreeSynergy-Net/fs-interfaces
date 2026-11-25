//! `FakeFixture`: the fixture of the suite for the fake (and for any module that serves the same board): the tables stand ONCE here.
//!
//! Pattern: Template Method (the fixture of the suite: client, level, second connection and the tables of the fake board; see `FormSourceFixture`).
//! Used by the own tests and, as the subject of the suite, by `fs-rpc` for `Guarded<FakeCore>` (the client is any `form_source::Client`).

use fs_interfaces::formsource_capnp::form_source;

use crate::{
    BlobSample, DeclaredCommand, DeclaredForm, FakeSource, FormSourceFixture, LaneCase, Ordered,
    Sample, ValueField, ValueKind,
};

/// Pattern: Template Method - the fixture of the fake board: its client, the level it says, whether it has an image, how many boards, a second connection.
#[derive(Clone)]
pub struct FakeFixture {
    client: form_source::Client,
    second: Option<form_source::Client>,
    level: u32,
    blob: bool,
    boards: usize,
}

impl FakeFixture {
    /// The fixture of `client`: level 2, no image, one board, no second connection.
    #[must_use]
    pub fn new(client: form_source::Client) -> Self {
        Self {
            client,
            second: None,
            level: 2,
            blob: false,
            boards: 1,
        }
    }

    /// A second connection to the SAME module (another program).
    #[must_use]
    pub fn with_second(mut self, second: form_source::Client) -> Self {
        self.second = Some(second);
        self
    }

    /// The level the suite believes the service speaks.
    #[must_use]
    pub fn with_level(mut self, level: u32) -> Self {
        self.level = level;
        self
    }

    /// The service has an image field (`image` of card `c1`).
    #[must_use]
    pub fn with_blob(mut self, blob: bool) -> Self {
        self.blob = blob;
        self
    }

    /// The service has `boards` boards.
    #[must_use]
    pub fn with_boards(mut self, boards: usize) -> Self {
        self.boards = boards;
        self
    }
}

impl FormSourceFixture for FakeFixture {
    fn client(&self) -> form_source::Client {
        self.client.clone()
    }
    fn level(&self) -> u32 {
        self.level
    }
    fn second(&self) -> Option<form_source::Client> {
        self.second.clone()
    }
    fn provider(&self) -> Option<&str> {
        Some(fs_interfaces::module::SAMPLE_PROVIDER)
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
        self.blob.then(|| BlobSample {
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
            accepted: self.boards == 1,
        })
    }
}
