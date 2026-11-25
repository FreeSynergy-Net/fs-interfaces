//! The data model of the catalog: forms, channels, versions. Pattern: Value Object.

/// How far a form is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// The form exists.
    Stable,
    /// Named, not built yet.
    Planned,
}

/// The kind of value a channel takes; decides which fs-types basic types may fill it ([`Art::allowed_basics`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Art {
    /// The set of records itself (a record type, no field).
    Records,
    /// Text: a name, a title, a value shown as text, or a value that sorts records into groups (column, lane, state).
    Text,
    /// Yes / no.
    Flag,
    /// A command of the module (`commands.json`), no field.
    Command,
    /// A point in time.
    Time,
    /// A reference to another record, a picture or a file.
    Reference,
    /// A quantity or a position in a sequence.
    Number,
}

impl Art {
    /// The word in `forms.txt`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Art::Records => "records",
            Art::Text => "text",
            Art::Flag => "flag",
            Art::Command => "command",
            Art::Time => "time",
            Art::Reference => "reference",
            Art::Number => "number",
        }
    }

    pub(super) fn parse(word: &str) -> Option<Art> {
        [
            Art::Records,
            Art::Text,
            Art::Flag,
            Art::Command,
            Art::Time,
            Art::Reference,
            Art::Number,
        ]
        .into_iter()
        .find(|a| a.name() == word)
    }

    /// The fs-types basic types (names as in `fs_forms_bind::Basic`) a field may have to fill a channel of this art. Empty for
    /// `Records` and `Command`: those channels name a record type or a command, not a field.
    #[must_use]
    pub fn allowed_basics(self) -> &'static [&'static str] {
        match self {
            Art::Records | Art::Command => &[],
            Art::Text => &["Text", "Email", "Choice", "Keywords", "Link", "Reference"],
            Art::Flag => &["Bool"],
            Art::Number => &["Number"],
            Art::Time => &["Date", "DateTime"],
            Art::Reference => &["Reference", "Image", "Link"],
        }
    }
}

/// How many fields a module maps onto a channel: from `min` to `max` (`None` = no upper limit). Written `1`, `0..1`, `0..4`, `0..n`
/// or `1..n` in `forms.txt`; a `?` (not decided) is refused by the reader. A required channel has `min >= 1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Count {
    /// Fewest fields.
    pub min: u32,
    /// Most fields.
    pub max: Option<u32>,
}

impl Count {
    /// Whether `n` fields on the channel are within the count (`min <= n` and `n <= max`).
    #[must_use]
    pub fn allows(&self, n: usize) -> bool {
        u32::try_from(n).is_ok_and(|n| n >= self.min && self.max.is_none_or(|m| n <= m))
    }
}

/// One channel of a form.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    /// The channel name (`items`, `label` ...).
    pub name: String,
    /// The kinds of value the channel takes: a set of 1 to 7 different arts (one element for most channels, `text|number|time|flag`
    /// for `facts`). A field fits when its art is in the set: [`Channel::accepts`].
    pub arts: Vec<Art>,
    /// A module must fill it (`!` in the file).
    pub required: bool,
    /// How many values.
    pub count: Count,
    /// Only for a channel of art `command`: the channels of the same form the gesture writes (5th field, `column! order lane`).
    /// `None` = the line has no 5th field (old lines stay readable); `Some(empty)` = writes nothing (e.g. remove).
    pub writes: Option<Vec<Write>>,
}

/// One entry of the `writes` column: a channel of the form the command writes. `required` (`!`) = the gesture must deliver it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Write {
    /// The channel name (not a command channel).
    pub channel: String,
    /// The gesture must deliver a value (`!`).
    pub required: bool,
}

/// The version of a form: `SemVer` `Major.Minor.Patch` (the crate `semver`, no adapter). Minor when an optional channel comes, Major when a channel
/// goes or changes meaning, Patch never (no change of the interface). Read with [`parse_version`].
pub use semver::Version;

/// Reads `H.N.P` (three plain numbers, no pre-release, no build part). `0.1` is no version and is refused.
#[must_use]
pub fn parse_version(text: &str) -> Option<Version> {
    let v = Version::parse(text).ok()?;
    (v.pre.is_empty() && v.build.is_empty()).then_some(v)
}

pub(super) fn number(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Pattern: Bridge – die Form trennt Datenart (Modul, Kanal→Feld in forms.json) und Darstellung (Component, `name@<req>`); beide ändern sich unabhängig.
/// Abweichung: Brücke als Daten (Tabelle), die Übersetzung macht fs-forms-run `args_of`, nicht der Dienst.
/// A form of the catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Form {
    /// The form name without version (`tree`).
    pub name: String,
    /// Its version.
    pub version: Version,
    /// The name for people.
    pub display_name: String,
    /// Whether it is built.
    pub status: Status,
    /// One line for people.
    pub description: String,
    /// The channels in file order.
    pub channels: Vec<Channel>,
    /// How channels belong together (lines `= ...`, see [`Relation`]); empty for most forms.
    pub relations: Vec<Relation>,
}

impl Channel {
    /// Whether a field of this art may fill the channel (`art` is in the set).
    #[must_use]
    pub fn accepts(&self, art: Art) -> bool {
        self.arts.contains(&art)
    }

    /// Whether a field of this fs-types basic type (name as in `Art::allowed_basics`) may fill the channel: some art of the set lists it.
    #[must_use]
    pub fn accepts_basic(&self, basic: &str) -> bool {
        self.arts
            .iter()
            .any(|a| a.allowed_basics().contains(&basic))
    }
}

impl Form {
    /// The key `name@Major.Minor.Patch`. Same as [`key`].
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }

    /// A channel by name.
    #[must_use]
    pub fn channel(&self, name: &str) -> Option<&Channel> {
        self.channels.iter().find(|c| c.name == name)
    }

    /// The channels a module has to fill.
    pub fn required(&self) -> impl Iterator<Item = &Channel> {
        self.channels.iter().filter(|c| c.required)
    }
}

/// How channels of one form belong together: a line `= <kind> <channel> ...` after the channels of the form.
///
/// Pattern: Value Object - a relation is data a reader checks (the channels exist and have the art the kind needs); the kinds are a closed,
/// named set. Deviation: none. New kinds only append (`#[non_exhaustive]`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Relation {
    /// `= interval start end all-day`: two `time` channels form the half-open interval `[start, end)` (ISO 8601-1; RFC 8984 `start` plus a
    /// never negative `duration`, zero length allowed): `end >= start`. With the optional `flag` channel set, both are `full-date` (RFC 8984
    /// `showWithoutTime`).
    Interval {
        /// The channel of the start (`time`, one value).
        start: String,
        /// The channel of the end (`time`, one value).
        end: String,
        /// The channel that says "whole days" (`flag`, one value), if the form has one.
        all_day: Option<String>,
    },
    /// `= parent parent`: the `reference` channel points to a record of the same form (a tree; `ActivityStreams` 2 `inReplyTo`/`context`).
    Parent {
        /// The channel of the reference (`reference`, one value).
        channel: String,
    },
}
