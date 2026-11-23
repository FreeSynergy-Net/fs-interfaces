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

/// A form of the catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// The key `name@Major.Minor.Patch`.
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
