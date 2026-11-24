//! `FakeSource`: a small, complete `FormSource` of level 1 with example data (a board with cards).
//!
//! Pattern: Fake Object (a small, complete source with example data; see the type).

use std::cell::RefCell;

use fs_interfaces::formsource_capnp::form_source;

use crate::fixture::{CommandKind, DeclaredCommand, DeclaredForm, Param, Sample, Verify};

/// How the fake behaves; every mode but `Strict` breaks one rule on purpose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// All rules of level 1 hold.
    Strict,
    /// Breaks the error channel: a refusal at `invoke` is an RPC exception, not `failed = true` + `problem`.
    LaxErrors,
    /// Breaks the shape of a refusal: a foreign problem type, `title` = the kind (no human text), and a `result` left on a refused write.
    LaxShape,
}

/// The form the fake serves.
pub const FORM: &str = "cards-in-columns@1.0.0";
pub(crate) const LOCKED: &str = "c3";
/// A command on the whole set that the asking person may NOT run (it is not in `RecordPage.may`).
pub(crate) const SET_LOCKED: &str = "card.clear";
/// The bytes of the image the fake serves for card `c1`, field `image` (when built `with_blob`).
pub const IMAGE: &[u8] = b"\x89PNG-fake-image-bytes";

/// Pattern: Fake Object (Test Double) – ein vollständiger, kleiner `FormSource` nach allen Regeln von Level 1; Modi brechen gezielt eine Regel.
pub struct FakeSource {
    pub(crate) mode: Mode,
    pub(crate) cards: RefCell<Vec<(String, String, String)>>,
    pub(crate) next: RefCell<u32>,
    pub(crate) blob: bool,
    pub(crate) boards: usize,
}

impl FakeSource {
    /// The fake with three cards (`c3` is locked: the asking person may not change it).
    #[must_use]
    pub fn new(mode: Mode) -> Self {
        let card =
            |id: &str, title: &str, col: &str| (id.to_owned(), title.to_owned(), col.to_owned());
        Self {
            mode,
            cards: RefCell::new(vec![
                card("c1", "One", "todo"),
                card("c2", "Two", "todo"),
                card(LOCKED, "Three", "done"),
            ]),
            next: RefCell::new(4),
            blob: false,
            boards: 1,
        }
    }

    /// The same fake, but with an image field (`image` of card `c1`): `blob` answers instead of `unsupported`.
    #[must_use]
    pub fn with_blob(mut self) -> Self {
        self.blob = true;
        self
    }

    /// The same fake with `n` boards (default 1). A create without `lane` is only done with exactly one board.
    #[must_use]
    pub fn with_boards(mut self, n: usize) -> Self {
        self.boards = n;
        self
    }

    /// One command, decided without any RPC: the record made or changed (empty for a delete) and a short text, or the refusal as a VALUE
    /// (kind, status, human text). A host shell turns the refusal into its own problem type; the fake needs no `fs-rpc` for that.
    ///
    /// # Errors
    /// The [`Refusal`] the service gives for this call.
    pub fn try_invoke(
        &self,
        command: &str,
        record: &str,
        args: &[(String, String)],
    ) -> Result<(String, &'static str), Refusal> {
        self.run(command, record, args)
    }

    /// A client of a new fake in this process.
    #[must_use]
    pub fn client(mode: Mode) -> form_source::Client {
        capnp_rpc::new_client(Self::new(mode))
    }

    /// The forms the fake declares.
    #[must_use]
    pub fn forms() -> Vec<DeclaredForm> {
        vec![DeclaredForm {
            key: FORM.into(),
            scope: String::new(),
            fields: vec![
                "title".into(),
                "column".into(),
                "image".into(),
                "due".into(),
            ],
        }]
    }

    /// The commands the fake declares.
    #[must_use]
    pub fn commands() -> Vec<DeclaredCommand> {
        let p = |n: &str, r: bool| Param {
            name: n.into(),
            required: r,
        };
        vec![
            DeclaredCommand {
                key: "card.move".into(),
                kind: CommandKind::Record,
                params: vec![p("column", true), p("after", false), p("before", false)],
            },
            DeclaredCommand {
                key: SET_LOCKED.into(),
                kind: CommandKind::Set,
                params: vec![],
            },
            DeclaredCommand {
                key: "card.delete".into(),
                kind: CommandKind::Record,
                params: vec![],
            },
            DeclaredCommand {
                key: "card.create".into(),
                kind: CommandKind::Set,
                params: vec![p("title", true), p("column", false)],
            },
        ]
    }

    /// Valid calls on the fake, in an order that works.
    #[must_use]
    pub fn samples() -> Vec<Sample> {
        let a = |k: &str, v: &str| (k.to_owned(), v.to_owned());
        vec![
            Sample {
                command: "card.move".into(),
                record: "c1".into(),
                args: vec![a("column", "done")],
                makes_record: true,
                verify: Verify::Value {
                    form: FORM.into(),
                    record: None,
                    path: "column".into(),
                    text: "done".into(),
                },
            },
            Sample {
                command: "card.create".into(),
                record: String::new(),
                args: vec![a("title", "Made"), a("column", "todo")],
                makes_record: true,
                verify: Verify::Value {
                    form: FORM.into(),
                    record: None,
                    path: "title".into(),
                    text: "Made".into(),
                },
            },
            Sample {
                command: "card.delete".into(),
                record: "c2".into(),
                args: vec![],
                makes_record: false,
                verify: Verify::Gone {
                    form: FORM.into(),
                    record: "c2".into(),
                },
            },
        ]
    }

    pub(crate) fn kind_of(command: &str) -> Option<CommandKind> {
        Self::commands()
            .iter()
            .find(|c| c.key == command)
            .map(|c| c.kind)
    }
}

pub(crate) fn refuse(kind: &str, status: u16, detail: &str) -> Refusal {
    let title = match kind {
        "unknown-form" => "This form is not known",
        "unknown-scope" => "This scope is not known",
        "unknown-command" => "This command is not known",
        "unknown-record" => "This record is not known",
        "forbidden" => "You may not do this",
        "bad-arguments" => "The arguments are not right",
        "unsupported" => "This is not supported",
        _ => "The service could not do this",
    };
    Refusal {
        kind: format!("form-source-{kind}"),
        status,
        title: title.to_owned(),
        detail: detail.to_owned(),
    }
}

/// A refusal as a value: the problem kind (`form-source-<kind>`), the HTTP-like status, a short text for people and the detail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// `form-source-<kind>` (the problem type of the contract).
    pub kind: String,
    /// The status of the table in `formsource.capnp`.
    pub status: u16,
    /// A short text for people (RFC 9457 `title`), never the kind.
    pub title: String,
    /// What exactly was wrong.
    pub detail: String,
}

impl Refusal {
    pub(crate) fn error(&self) -> capnp::Error {
        capnp::Error::failed(format!("{}: {} ({})", self.kind, self.detail, self.status))
    }
}
