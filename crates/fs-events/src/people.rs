//! `FakePeople`: the people directory the import consumes, with a few fixed (invented) people and a "proposed" list.
//!
//! Pattern: Fake behind a small own port. The form is the consumed part of `person_directory_v1` (fs-people, still a
//! draft: `find_by_address`, `open_batch`, `propose(ProposeEntry, batch)`); it is NOT imported from there. When
//! fs-people has a tag, [`Directory`] is replaced by that trait and nothing else changes. Names live here, never in an
//! appointment.

use std::sync::Mutex;

use async_trait::async_trait;

use crate::draft::{ContactAddress, Error, ObjectId, PersonArt, PersonRef, Reference, Result};
use crate::ids::{object_id, person, person_iri};

/// A new entry in the state "proposed" (nobody confirmed it); `origin` = where it came from (file, account).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposeEntry {
    /// The name as the source gave it; lands only in the entry.
    pub display_hint: String,
    /// The address, if the source had one.
    pub address: Option<ContactAddress>,
    /// The origin of the proposal (`urn:sha256:...` of a file, an account IRI).
    pub origin: Option<Reference>,
}

/// The consumed part of the directory. An AI asking with an address is `Forbidden` (no address oracle).
#[async_trait]
pub trait Directory: Send + Sync {
    /// `person.find`: the entries with exactly this address, in the order of the directory (first = the one to take).
    async fn find_by_address(&self, address: &ContactAddress) -> Result<Vec<PersonRef>>;
    /// `person.propose`: open a batch for one bulk proposal (the import of a file).
    async fn open_batch(&self, origin: &Reference) -> Result<ObjectId>;
    /// `person.propose`: add an entry, optionally in an open batch; a known address gives the existing reference.
    async fn propose(&self, entry: &ProposeEntry, batch: Option<ObjectId>) -> Result<PersonRef>;
    /// Stand-in for `person.excerpt` with the purpose "export": name and address of a person for the `CN` and the
    /// `mailto:` of an iCalendar file; `None` when there is nothing to show.
    async fn display(&self, person: &PersonRef) -> Result<Option<(String, Option<String>)>>;
}

/// One known person.
#[derive(Clone, Debug)]
pub struct Known {
    /// The reference appointments hold.
    pub person: PersonRef,
    /// Display name (the only place the name exists).
    pub name: String,
    /// Address as `mailto:...`.
    pub address: String,
}

/// An entry made by `propose`.
#[derive(Clone, Debug)]
pub struct Proposed {
    /// What was proposed.
    pub known: Known,
    /// The batch it went into.
    pub batch: Option<ObjectId>,
    /// The origin it was given.
    pub origin: Option<Reference>,
}

#[derive(Debug, Default)]
struct Inner {
    proposed: Vec<Proposed>,
    batches: Vec<(ObjectId, Reference)>,
    next: u64,
}

/// The directory.
#[derive(Debug)]
pub struct FakePeople {
    known: Vec<Known>,
    inner: Mutex<Inner>,
    ai: bool,
}

impl FakePeople {
    /// A directory with exactly these people.
    #[must_use]
    pub fn new(known: Vec<Known>) -> Self {
        let inner = Inner {
            next: 900,
            ..Inner::default()
        };
        Self {
            known,
            inner: Mutex::new(inner),
            ai: false,
        }
    }

    /// The asker is an AI: `find_by_address` and `propose` with an address are `Forbidden`.
    #[must_use]
    pub fn with_ai(mut self) -> Self {
        self.ai = true;
        self
    }

    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Entries made by `propose` (state "proposed", nobody confirmed them).
    #[must_use]
    pub fn proposed(&self) -> Vec<Proposed> {
        self.inner().proposed.clone()
    }

    /// The batches opened so far (id and origin).
    #[must_use]
    pub fn batches(&self) -> Vec<(ObjectId, Reference)> {
        self.inner().batches.clone()
    }

    /// Make a known person (helper for the sample).
    #[must_use]
    pub fn known(n: u64, name: &str, address: &str) -> Known {
        Known {
            person: person(n),
            name: name.to_owned(),
            address: address.to_owned(),
        }
    }

    fn hits(&self, address: &str) -> Vec<PersonRef> {
        let inner = self.inner();
        self.known
            .iter()
            .chain(inner.proposed.iter().map(|x| &x.known))
            .filter(|k| k.address.eq_ignore_ascii_case(address))
            .map(|k| k.person.clone())
            .collect()
    }
}

fn forbidden(capability: &str) -> Error {
    Error::Forbidden {
        capability: capability.to_owned(),
    }
}

#[async_trait]
impl Directory for FakePeople {
    async fn find_by_address(&self, address: &ContactAddress) -> Result<Vec<PersonRef>> {
        if self.ai {
            return Err(forbidden("person.find"));
        }
        Ok(self.hits(&address.canonical()))
    }

    async fn open_batch(&self, origin: &Reference) -> Result<ObjectId> {
        let mut inner = self.inner();
        inner.next += 1;
        let id = object_id(inner.next);
        inner.batches.push((id, origin.clone()));
        Ok(id)
    }

    async fn display(&self, p: &PersonRef) -> Result<Option<(String, Option<String>)>> {
        let inner = self.inner();
        Ok(self
            .known
            .iter()
            .chain(inner.proposed.iter().map(|x| &x.known))
            .find(|k| k.person.id == p.id)
            .map(|k| {
                (
                    k.name.clone(),
                    Some(k.address.clone()).filter(|a| !a.is_empty()),
                )
            }))
    }

    async fn propose(&self, entry: &ProposeEntry, batch: Option<ObjectId>) -> Result<PersonRef> {
        if self.ai && entry.address.is_some() {
            return Err(forbidden("person.propose"));
        }
        if let Some(a) = &entry.address {
            if let Some(found) = self.hits(&a.canonical()).into_iter().next() {
                return Ok(found);
            }
        } else if entry.display_hint.trim().is_empty() {
            return Err(Error::Invalid(Vec::new()));
        }
        let mut inner = self.inner();
        inner.next += 1;
        let p = PersonRef {
            id: person_iri(inner.next),
            art: PersonArt::Profile,
            abzug: None,
        };
        inner.proposed.push(Proposed {
            known: Known {
                person: p.clone(),
                name: entry.display_hint.clone(),
                address: entry
                    .address
                    .as_ref()
                    .map(ContactAddress::canonical)
                    .unwrap_or_default(),
            },
            batch,
            origin: entry.origin.clone(),
        });
        Ok(p)
    }
}
