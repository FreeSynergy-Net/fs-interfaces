//! `FakePeople`: the consumed side of fs-people in memory (`PersonDirectory`, `PersonFacts`, `RoleBook` of the
//! contract folder), shared by a `FakeGroups` and its tests.
//!
//! Pattern: Fake (a test double with working behaviour) behind three small consumed traits (Adapter port).

use crate::groups::shared::{PersonArt, PersonRef};
use crate::groups::{PersonDirectory, PersonFacts, Result, RoleBook, RoleInfo, RoleRef};
use crate::ids::{key, object_id, reference_of};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

/// First number of the persons a fake makes (the groups count from 1).
const PERSON_BASE: u64 = 1_000_000;
/// First number of the roles a fake makes.
const ROLE_BASE: u64 = 2_000_000;

/// What the fake knows of one person.
#[derive(Clone, Debug)]
struct Entry {
    person: PersonRef,
    display: String,
    address: Option<String>,
    /// `Some(true)` an AI account, `Some(false)` a human, `None` not known.
    agent: Option<bool>,
    /// `Some(true)` a supervised account (ADR 0095 Pkt. 14), `Some(false)` not, `None` not known.
    supervised: Option<bool>,
}

#[derive(Debug, Default)]
struct Book {
    persons: BTreeMap<String, Entry>,
    roles: Vec<RoleInfo>,
    next_person: u64,
    next_role: u64,
}

/// In-memory people, roles and the AI mark.
#[derive(Debug, Default)]
pub struct FakePeople {
    book: Mutex<Book>,
}

impl FakePeople {
    /// No persons and no roles.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, Book> {
        self.book
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn add(&self, display: &str, address: Option<&str>, agent: Option<bool>) -> PersonRef {
        let mut book = self.lock();
        book.next_person += 1;
        let person = PersonRef {
            id: reference_of(object_id(PERSON_BASE + book.next_person)),
            art: PersonArt::Person,
            abzug: None,
        };
        Self::put(&mut book, person, display, address, agent)
    }

    fn put(
        book: &mut Book,
        person: PersonRef,
        display: &str,
        address: Option<&str>,
        agent: Option<bool>,
    ) -> PersonRef {
        let entry = Entry {
            person: person.clone(),
            display: display.to_owned(),
            address: address.map(str::to_owned),
            agent,
            supervised: (agent == Some(false)).then_some(false),
        };
        book.persons.insert(key(&person), entry);
        person
    }

    /// A person that IS entry `n` of the directory sample (`fs_person_directory::sample()`): its reference and its
    /// display name come from there, so the same IRI means the same person in every sample. `None` for a number
    /// that is not a person of that sample. `agent` is the mark (`Some(true)` for an AI account such as entry 4).
    pub fn from_directory(&self, n: u8, agent: Option<bool>) -> Option<PersonRef> {
        let person = fs_person_directory::sample_ref(n).filter(|p| p.art == PersonArt::Person)?;
        let display = fs_person_directory::sample_name(n)?;
        let address = fs_person_directory::sample_address(n);
        Some(Self::put(&mut self.lock(), person, display, address, agent))
    }

    /// A new human (marked as no AI).
    pub fn human(&self, display: &str) -> PersonRef {
        let address = format!(
            "mailto:{}@example.org",
            display.to_lowercase().replace(' ', ".")
        );
        self.add(display, Some(&address), Some(false))
    }

    /// A new AI account.
    pub fn agent(&self, display: &str) -> PersonRef {
        self.add(display, None, Some(true))
    }

    /// A new person nobody knows to be human or AI.
    pub fn unknown(&self, display: &str) -> PersonRef {
        self.add(display, None, None)
    }

    /// Set the mark: `Some(true)` AI, `Some(false)` human, `None` not known (the mark changes after the fact).
    /// `false` when the person is not known here.
    pub fn mark(&self, person: &PersonRef, agent: Option<bool>) -> bool {
        self.lock()
            .persons
            .get_mut(&key(person))
            .map(|e| e.agent = agent)
            .is_some()
    }

    /// Whether `person` is known here.
    #[must_use]
    pub fn knows(&self, person: &PersonRef) -> bool {
        self.lock().persons.contains_key(&key(person))
    }

    /// The mark of `person`: `Some(true)` AI, `Some(false)` human, `None` not known or not marked.
    #[must_use]
    pub fn status(&self, person: &PersonRef) -> Option<bool> {
        self.lock().persons.get(&key(person)).and_then(|e| e.agent)
    }

    /// Set the supervision mark: `Some(true)` supervised, `Some(false)` not, `None` not known. `false` when the person
    /// is not known here.
    pub fn supervise(&self, person: &PersonRef, supervised: Option<bool>) -> bool {
        self.lock()
            .persons
            .get_mut(&key(person))
            .map(|e| e.supervised = supervised)
            .is_some()
    }

    /// The supervision mark of `person` (`None` not known or not marked).
    #[must_use]
    pub fn supervised(&self, person: &PersonRef) -> Option<bool> {
        self.lock()
            .persons
            .get(&key(person))
            .and_then(|e| e.supervised)
    }

    /// A new role in the book; `admin` marks an admin role.
    pub fn add_role(&self, label: &str, admin: bool) -> RoleRef {
        let mut book = self.lock();
        book.next_role += 1;
        let role = RoleRef(reference_of(object_id(ROLE_BASE + book.next_role)));
        book.roles.push(RoleInfo {
            role: role.clone(),
            label: label.to_owned(),
            admin,
        });
        role
    }

    /// The role with this reference, when the book has it.
    #[must_use]
    pub fn role_info(&self, role: &RoleRef) -> Option<RoleInfo> {
        self.lock().roles.iter().find(|r| &r.role == role).cloned()
    }

    /// Put a person in with a fixed reference (the sample).
    pub fn insert(&self, person: PersonRef, display: &str, agent: Option<bool>) {
        let at = key(&person);
        let entry = Entry {
            person,
            display: display.to_owned(),
            address: None,
            agent,
            supervised: (agent == Some(false)).then_some(false),
        };
        self.lock().persons.insert(at, entry);
    }

    /// Put a role in with a fixed reference (the sample).
    pub fn insert_role(&self, role: RoleRef, label: &str, admin: bool) {
        self.lock().roles.push(RoleInfo {
            role,
            label: label.to_owned(),
            admin,
        });
    }
}

#[async_trait]
impl PersonDirectory for FakePeople {
    async fn find_by_address(&self, uri: &str) -> Result<Option<PersonRef>> {
        let book = self.lock();
        Ok(book
            .persons
            .values()
            .find(|e| e.address.as_deref() == Some(uri))
            .map(|e| e.person.clone()))
    }

    async fn propose(&self, display_hint: &str, uri: &str) -> Result<PersonRef> {
        if let Some(found) = self.find_by_address(uri).await? {
            return Ok(found);
        }
        Ok(self.add(display_hint, Some(uri), None))
    }

    async fn display(&self, person: &PersonRef) -> Result<Option<(String, Option<String>)>> {
        let book = self.lock();
        Ok(book
            .persons
            .get(&key(person))
            .map(|e| (e.display.clone(), e.address.clone())))
    }
}

#[async_trait]
impl PersonFacts for FakePeople {
    async fn is_agent(&self, person: &PersonRef) -> Result<Option<bool>> {
        Ok(self.status(person))
    }
    async fn is_supervised(&self, person: &PersonRef) -> Result<Option<bool>> {
        Ok(self.supervised(person))
    }
}

#[async_trait]
impl RoleBook for FakePeople {
    async fn roles(&self) -> Result<Vec<RoleInfo>> {
        Ok(self.lock().roles.clone())
    }

    async fn role(&self, role: &RoleRef) -> Result<Option<RoleInfo>> {
        Ok(self.role_info(role))
    }
}
