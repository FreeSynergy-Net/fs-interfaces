//! The `LocalOperator` impl of `FakeGroups` and its test helpers: the way the local command of the server program
//! reaches the provider. NOT part of `GroupGovernance` (ONLY LOCAL, see `groups/operator.rs`).
//!
//! Pattern: Adapter (the async trait on one side, the sync state operations on the other).

use crate::fake::FakeGroups;
use crate::groups::shared::PersonRef;
use crate::groups::{LocalOperator, Result};
use async_trait::async_trait;

#[async_trait]
impl LocalOperator for FakeGroups {
    async fn operator_set_local(&self, person: &PersonRef) -> Result<()> {
        self.lock().operator_set_local(person)
    }
    async fn operator_unset_local(&self, person: &PersonRef) -> Result<()> {
        self.lock().operator_unset_local(person)
    }
}

impl FakeGroups {
    /// Single-user setup (ADR 0090): the one person is the operator, as the local command made her at the setup.
    ///
    /// # Panics
    /// When `person` is no known, unsupervised human (the setup is wrong then; the same checks as the local command).
    #[must_use]
    pub fn with_operator(self, person: &PersonRef) -> Self {
        self.lock()
            .operator_set_local(person)
            .expect("the operator of a single-user setup is a known human");
        self
    }

    /// Put an operator in as it is: no checks, no log (old data, a mark that changes later; for tests).
    pub fn insert_operator_raw(&self, person: &PersonRef) {
        self.lock().operator_insert_raw(person);
    }

    /// Everybody in the operator group with the role, marks ignored (for tests of the fail-closed read).
    #[must_use]
    pub fn operator_persons_raw(&self) -> Vec<PersonRef> {
        self.lock().operator_persons_raw()
    }
}
