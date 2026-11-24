//! `FormSourceFixture`: what a service under test tells the suite (its client, its declared forms and commands, valid calls).
//!
//! Pattern: Template Method (the rules are fixed, the fixture supplies client, declared forms and valid calls).

use fs_interfaces::formsource_capnp::form_source;

/// Pattern: Template Method – die Regeln der Suite stehen fest, die Fixture liefert nur Client, erklärte Formen und gültige Beispielaufrufe.
pub trait FormSourceFixture {
    /// The service under test.
    fn client(&self) -> form_source::Client;

    /// The level the service speaks (`hello`). Level 1 = the rules L2-* are skipped, never passed; from 2 they run.
    fn level(&self) -> u32 {
        1
    }

    /// A second connection to the SAME module: it stands for another program (`None` = rules L2-8 and L2-11 skipped).
    fn second(&self) -> Option<form_source::Client> {
        None
    }

    /// Whether the service can black out a change (`false` = the rule "gone" of L2-9 is skipped).
    fn can_redact(&self) -> bool {
        false
    }

    /// The `provider` the service must say in `hello` (`None` = any non-empty text).
    fn provider(&self) -> Option<&str> {
        None
    }

    /// The forms the module declares (`forms.json`): key, scope to read, its fields.
    fn forms(&self) -> Vec<DeclaredForm>;

    /// The commands the module declares (`commands.json`).
    fn commands(&self) -> Vec<DeclaredCommand>;

    /// One valid call per command the suite may run (they change data, in this order).
    fn samples(&self) -> Vec<Sample>;

    /// Whether the service refuses a scope it does not know (`false` = rule skipped).
    fn strict_scope(&self) -> bool {
        false
    }

    /// A command and a record the asking person may NOT change (`None` = rule skipped).
    fn forbidden(&self) -> Option<(String, String)> {
        None
    }

    /// A command on the whole set that the asking person may NOT run (not in `RecordPage.may`; `None` = rule skipped).
    fn forbidden_set(&self) -> Option<String> {
        None
    }

    /// Whether some form has an image or file field (`false` = `blob` must say `form-source-unsupported`).
    fn has_blob(&self) -> bool {
        self.blob_sample().is_some()
    }

    /// Fields of art `time` as (form key, path): every value read must be an RFC 3339 `date-time` with an offset or `Z`, never a local time.
    fn time_fields(&self) -> Vec<(String, String)> {
        Vec::new()
    }

    /// A create without the optional `lane` (`None` = skipped); see [`LaneCase`].
    fn create_without_lane(&self) -> Option<LaneCase> {
        None
    }

    /// One image or file field with content (`None` = rule 20 skipped).
    fn blob_sample(&self) -> Option<BlobSample> {
        None
    }

    /// A move in an ordered set by neighbours (`None` = rule 21 skipped).
    fn ordered(&self) -> Option<Ordered> {
        None
    }
}

/// A field with bytes: the record and the field name for `blob`.
#[derive(Clone, Debug)]
pub struct BlobSample {
    /// The record.
    pub record: String,
    /// The field name.
    pub field: String,
}

/// A command that places a record by its neighbours: `args` are the other required arguments (for example the column).
#[derive(Clone, Debug)]
pub struct Ordered {
    /// The contract key of the move command.
    pub command: String,
    /// The form to read the order from.
    pub form: String,
    /// The record that moves.
    pub record: String,
    /// The record it is placed next to (it must exist and differ from `record`).
    pub neighbour: String,
    /// The other required arguments.
    pub args: Vec<(String, String)>,
}

/// A declared form: key `name@x.y.z`, the scope to read it with (empty = all) and the field names (dotted paths) its records may carry.
#[derive(Clone, Debug)]
pub struct DeclaredForm {
    /// `name@x.y.z`.
    pub key: String,
    /// The scope the suite reads with.
    pub scope: String,
    /// The field names of the module.
    pub fields: Vec<String>,
}

/// On what a command works.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKind {
    /// On one record (listed in `RecordView.may`).
    Record,
    /// On the set (listed in `RecordPage.may`).
    Set,
}

/// One parameter of a command; `required` must be given.
#[derive(Clone, Debug)]
pub struct Param {
    /// The field name.
    pub name: String,
    /// Must be given.
    pub required: bool,
}

/// A declared command by its contract key (`card.move`, not the host id).
#[derive(Clone, Debug)]
pub struct DeclaredCommand {
    /// The contract key.
    pub key: String,
    /// Record or set.
    pub kind: CommandKind,
    /// Its parameters.
    pub params: Vec<Param>,
}

/// What a read must show after a sample ran.
#[derive(Clone, Debug)]
pub enum Verify {
    /// Nothing to read back.
    None,
    /// The record (`None` = the one named in `result.record`) carries `text` at `path` in the form `form`.
    Value {
        form: String,
        record: Option<String>,
        path: String,
        text: String,
    },
    /// The record is gone from the form.
    Gone { form: String, record: String },
}

/// One valid call and its effect.
#[derive(Clone, Debug)]
pub struct Sample {
    /// The contract key.
    pub command: String,
    /// The record (empty for a set command).
    pub record: String,
    /// The arguments by field name.
    pub args: Vec<(String, String)>,
    /// `true`: `result.record` must be set (not for a delete).
    pub makes_record: bool,
    /// The read-back.
    pub verify: Verify,
}

/// A create command called WITHOUT `lane`. Rule: exactly ONE board = that board (`accepted`); no or several boards = `bad-arguments` 400 with a
/// text for people, never a silent guess.
#[derive(Clone, Debug)]
pub struct LaneCase {
    /// The contract key of the create command.
    pub command: String,
    /// The arguments (no `lane`).
    pub args: Vec<(String, String)>,
    /// `true`: the service has exactly one board, the create is done. `false`: none or several, it is refused.
    pub accepted: bool,
}
