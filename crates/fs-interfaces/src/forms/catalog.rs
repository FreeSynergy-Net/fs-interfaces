//! The built-in catalog and its lookups. Pattern: Registry (a fixed table), like `capabilities`.

use std::sync::OnceLock;

use super::{Form, Version, parse, parse_version};

/// The file as written in `schema/forms.txt`.
pub const SOURCE: &str = include_str!("../../../../schema/forms.txt");

/// The level of the `FormSource` handshake (`hello`): a plain number, never a name with a version suffix. Levels are CUMULATIVE: level n can do everything of level n-1.
/// A higher level only APPENDS (methods, fields). Something that breaks is not a new level: it raises `FORM_SOURCE_MIN_LEVEL` or is a new
/// contract. A host takes every level `>= FORM_SOURCE_MIN_LEVEL` and uses `min(level, FORM_SOURCE_LEVEL)`, so a newer service
/// still works with it (only what the host knows is used); a service below the minimum is refused with the reason.
/// Level 1 = `hello`/`page`/`blob`/`invoke` with the rules written in `formsource.capnp`; Level 2 = Stufe 3: heeds `expect`, `step`, `introduce`, `watch` (`invokeIf`, `revert`, `changes`, `introduce`, `watch`).
pub const FORM_SOURCE_LEVEL: u32 = 2;

/// The oldest level a host still accepts. A host takes every level `>= FORM_SOURCE_MIN_LEVEL`, works with `min(level, FORM_SOURCE_LEVEL)` and uses `invokeIf`, `revert`,
/// `changes`, `introduce` and `watch` only from level 2. A service says its OWN level, never this constant.
pub const FORM_SOURCE_MIN_LEVEL: u32 = 1;

/// The forms of the built-in file, in file order. The file is checked by the tests of this crate; if it were bad the list is empty.
#[must_use]
pub fn catalog() -> &'static [Form] {
    static CATALOG: OnceLock<Vec<Form>> = OnceLock::new();
    CATALOG.get_or_init(|| parse(SOURCE).unwrap_or_default())
}

/// A form by name and exact version.
#[must_use]
pub fn find(name: &str, version: &Version) -> Option<&'static Form> {
    catalog()
        .iter()
        .find(|f| f.name == name && f.version == *version)
}

/// A form by its key `name@H.N.P`.
#[must_use]
pub fn find_key(key: &str) -> Option<&'static Form> {
    let (name, version) = key.split_once('@')?;
    find(name, &parse_version(version)?)
}

/// Reads a form key: `name@x.y.z` or the short form `x.y` (= `x.y.0`, tolerant reader). `None` for a missing name, a missing `@`,
/// a pre-release/build part or anything else that is no version. Does not look the form up (see [`find_key`]).
#[must_use]
pub fn parse_key(text: &str) -> Option<(&str, Version)> {
    let (name, version) = text.split_once('@')?;
    if name.is_empty() {
        return None;
    }
    let parsed = parse_version(version).or_else(|| {
        let (major, minor) = version.split_once('.')?;
        parse_version(&format!("{major}.{minor}.0"))
    })?;
    Some((name, parsed))
}

/// The key a writer sends: always `name@x.y.z`.
#[must_use]
pub fn key(name: &str, version: &Version) -> String {
    format!("{name}@{version}")
}
