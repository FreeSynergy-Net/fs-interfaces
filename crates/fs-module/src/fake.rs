//! `FakeSource`: a small, complete `FormSource` of level 1 with example data (a board with cards).

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
}

/// The form the fake serves.
pub const FORM: &str = "cards-in-columns@1.0.0";
pub(crate) const LOCKED: &str = "c3";

/// Pattern: Fake Object (Test Double) – ein vollständiger, kleiner FormSource nach allen Regeln von Level 1; Modi brechen gezielt eine Regel.
pub struct FakeSource {
    pub(crate) mode: Mode,
    pub(crate) cards: RefCell<Vec<(String, String, String)>>,
    pub(crate) next: RefCell<u32>,
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
        }
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
            fields: vec!["title".into(), "column".into()],
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
                params: vec![p("column", true)],
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
                args: vec![a("title", "Made")],
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
    Refusal {
        kind: format!("form-source-{kind}"),
        status,
        detail: detail.to_owned(),
    }
}

pub(crate) struct Refusal {
    pub(crate) kind: String,
    pub(crate) status: u16,
    pub(crate) detail: String,
}

impl Refusal {
    pub(crate) fn error(&self) -> capnp::Error {
        capnp::Error::failed(format!("{}: {} ({})", self.kind, self.detail, self.status))
    }
}
