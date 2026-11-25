//! Deterministic ids, so a test can name a thing without asking the provider.
//!
//! Pattern: none (pure functions).

use crate::draft::{
    Attendee, ObjectId, Participant, PersonArt, PersonRef, Reference, Restorable, Utc,
};

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

/// The numbers of the directory sample (`fs_person_directory::sample()`) the sample people 1 to 4 are: entries 1, 10,
/// 12 and 4, all of the art Person (entries 2 and 3 are profiles). The map swaps 2 with 10 and 3 with 12, so it is its
/// own inverse and no two numbers give one IRI.
const fn directory_number(n: u64) -> u64 {
    match n {
        2 => 10,
        10 => 2,
        3 => 12,
        12 => 3,
        other => other,
    }
}

/// A local person IRI (`urn:uuid:...`) with the number `n`. The numbers 1 to 4 are the IRIs of the people 1, 10, 12
/// and 4 of the directory sample of `fs-person-directory` (same IRI = same person); the others are free.
#[must_use]
pub fn person_iri(n: u64) -> Reference {
    let n = directory_number(n);
    crate::zone::reference(&format!("urn:uuid:00000000-0000-7000-8000-{n:012x}"))
}

/// The number of the sample person `n` (1 to 4) in the directory sample, `None` for any other `n`.
#[must_use]
pub fn directory_entry(n: u64) -> Option<u8> {
    (1..=4)
        .contains(&n)
        .then(|| u8::try_from(directory_number(n)).ok())
        .flatten()
}

/// The IRI of the calendar with this id (what `Origin.source` of subscribed appointments holds, never the feed address).
#[must_use]
pub fn calendar_iri(id: ObjectId) -> Reference {
    crate::zone::reference(&format!(
        "urn:uuid:00000000-0000-7000-a000-{:012x}",
        object_number(id)
    ))
}

/// A reference to the person with the number `n` (kind `Person`, no excerpt).
#[must_use]
pub fn person(n: u64) -> PersonRef {
    PersonRef {
        id: person_iri(n),
        art: PersonArt::Person,
        abzug: None,
    }
}

/// The person a participant is, if it is a person, group or organisation (not a room or a device).
#[must_use]
pub fn actor(p: &Participant) -> Option<&PersonRef> {
    match &p.who {
        Attendee::Actor(person) => Some(person),
        // A room or device (`Resource`, `Location`) and any kind this crate does not know (`Attendee` is
        // `#[non_exhaustive]`) are not a person it can name.
        _ => None,
    }
}

/// The end of a `Restorable` that has one: `Until(t)` and `Held(Some(t))`; `None` for `Manual` and `Held(None)`.
#[must_use]
pub fn end_of(r: Restorable) -> Option<Utc> {
    match r {
        Restorable::Until(t) | Restorable::Held(Some(t)) => Some(t),
        _ => None,
    }
}
