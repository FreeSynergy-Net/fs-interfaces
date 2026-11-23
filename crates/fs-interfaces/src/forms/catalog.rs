//! The built-in catalog and its lookups. Pattern: Registry (a fixed table), like `capabilities`.

use std::sync::OnceLock;

use super::{Form, Version, parse, parse_version};

/// The file as written in `schema/forms.txt`.
pub const SOURCE: &str = include_str!("../../../../schema/forms.txt");

/// The level of the `FormSource` handshake (`hello`): a plain number, never a name with a version suffix. A caller refuses a
/// level it does not know. A change that breaks a reader raises it; a new method does not.
pub const FORM_SOURCE_LEVEL: u32 = 1;

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
