//! Test sources of the local owner and the contract check every source must pass (feature `fake`).
//!
//! Pattern: Strategy (test variants of [`LocalOwner`]) plus Template Method for [`check`]: the same three rules for
//! every source, the source is the only thing that varies.

use fs_types::{BasicValue, PersonArt, PersonRef};

use super::{LocalOwner, OwnerProblem};

/// A source that always answers the same owner.
#[derive(Clone, Debug)]
pub struct FixedOwner(pub PersonRef);

impl LocalOwner for FixedOwner {
    fn owner(&self) -> Result<PersonRef, OwnerProblem> {
        Ok(self.0.clone())
    }
}

/// A source that always answers the same problem.
#[derive(Clone, Debug)]
pub struct NoOwner(pub OwnerProblem);

impl LocalOwner for NoOwner {
    fn owner(&self) -> Result<PersonRef, OwnerProblem> {
        Err(self.0.clone())
    }
}

/// A valid sample owner: `urn:uuid:` with a UUIDv7, kind `Person`, no `abzug`.
///
/// # Panics
/// Never: the sample text is a valid reference.
#[must_use]
pub fn sample() -> PersonRef {
    person("urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e0f")
}

/// A person reference of kind `Person` for `iri` (for tests, also for invalid ones).
///
/// # Panics
/// If `iri` is not an absolute reference at all.
#[must_use]
pub fn person(iri: &str) -> PersonRef {
    PersonRef {
        id: fs_types::Reference::parse(iri).expect("absolute reference"),
        art: PersonArt::Person,
        abzug: None,
    }
}

/// The contract of every [`LocalOwner`]. Returns the first broken rule.
///
/// 1. `Ok` means kind `Person`, `abzug == None`, and an id `urn:uuid:` with a UUIDv7.
/// 2. Asked twice, the same id comes back.
/// 3. `Err` has a non-empty reason.
///
/// # Errors
/// The rule that is broken, in words.
pub fn check(source: &dyn LocalOwner) -> Result<(), String> {
    let first = source.owner();
    match &first {
        Ok(p) => rule_valid(p)?,
        Err(e) if e.reason().trim().is_empty() => {
            return Err(format!("rule 3: {} has an empty reason", e.code()));
        }
        Err(_) => {}
    }
    if first != source.owner() {
        return Err("rule 2: asked twice, a different answer".to_owned());
    }
    Ok(())
}

fn rule_valid(p: &PersonRef) -> Result<(), String> {
    if p.art != PersonArt::Person {
        return Err("rule 1: kind is not Person".to_owned());
    }
    if p.abzug.is_some() {
        return Err("rule 1: abzug is set".to_owned());
    }
    let id = p.id.canonical();
    let uuid = id
        .strip_prefix("urn:uuid:")
        .ok_or("rule 1: id does not start with urn:uuid:")?;
    if is_v7(uuid) {
        Ok(())
    } else {
        Err(format!("rule 1: {uuid} is not a UUIDv7"))
    }
}

/// 8-4-4-4-12 hex digits, version nibble 7, variant 10xx.
fn is_v7(u: &str) -> bool {
    let b = u.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                *c == b'-'
            } else {
                c.is_ascii_hexdigit()
            }
        })
        && b[14] == b'7'
        && matches!(b[19], b'8' | b'9' | b'a' | b'b' | b'A' | b'B')
}
