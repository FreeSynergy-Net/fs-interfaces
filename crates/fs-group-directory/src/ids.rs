//! Deterministic ids and references, so a test can name a thing without asking the provider.
//!
//! Pattern: none (pure functions).

use crate::groups::shared::{BasicValue, ObjectId, PersonArt, PersonRef, Reference};
use crate::groups::{Error, Missing};
use std::fmt::Write as _;

/// The object id with the number `n` (big-endian in the last eight bytes).
#[must_use]
pub fn object_id(n: u64) -> ObjectId {
    let mut bytes = [0_u8; 16];
    bytes[8..].copy_from_slice(&n.to_be_bytes());
    ObjectId(bytes)
}

/// The number an [`object_id`] was made from.
#[must_use]
pub fn object_number(id: ObjectId) -> u64 {
    let mut tail = [0_u8; 8];
    tail.copy_from_slice(&id.0[8..]);
    u64::from_be_bytes(tail)
}

/// The `urn:uuid:` reference of an id (group, person, role).
///
/// # Panics
/// Never: a `urn:uuid:` text is always a valid reference.
#[must_use]
pub fn reference_of(id: ObjectId) -> Reference {
    let hex = id.0.iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    });
    let text = format!(
        "urn:uuid:{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    );
    Reference::parse(&text).expect("a urn:uuid is a reference")
}

/// The id a `urn:uuid:` reference was made from (`None` for any other reference).
#[must_use]
pub fn id_of_reference(reference: &Reference) -> Option<ObjectId> {
    let canonical = reference.canonical();
    let hex: String = canonical
        .strip_prefix("urn:uuid:")?
        .chars()
        .filter(|c| *c != '-')
        .collect();
    if hex.len() != 32 {
        return None;
    }
    let mut bytes = [0_u8; 16];
    for (at, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(at * 2..at * 2 + 2)?, 16).ok()?;
    }
    Some(ObjectId(bytes))
}

/// A reference to the person with the number `n` (kind `Person`, no excerpt).
#[must_use]
pub fn person(n: u64) -> PersonRef {
    PersonRef {
        id: reference_of(object_id(n)),
        art: PersonArt::Person,
        abzug: None,
    }
}

/// A reference to the group with this id (kind `Group`).
#[must_use]
pub fn group_ref(id: ObjectId) -> PersonRef {
    PersonRef {
        id: reference_of(id),
        art: PersonArt::Group,
        abzug: None,
    }
}

/// The text a reference sorts by (references have no order of their own).
pub(crate) fn key(person: &PersonRef) -> String {
    person.id.canonical()
}

/// `NotFound` of a group.
pub(crate) fn no_group(id: ObjectId) -> Error {
    Error::NotFound(Missing::Group(id))
}
