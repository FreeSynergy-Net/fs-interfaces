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

/// A local person IRI (`urn:uuid:...`) with the number `n`.
#[must_use]
pub fn person_iri(n: u64) -> Reference {
    crate::zone::reference(&format!("urn:uuid:00000000-0000-7000-8000-{n:012x}"))
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
        Attendee::Resource(_) | Attendee::Location(_) => None,
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
