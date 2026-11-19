//! The canary test: the program under test, the arguments, the driver, and the list of what leaked.
//! Test helpers; no pattern of their own.
#![allow(dead_code)]

pub mod calls;
pub mod drive;
pub mod find;
pub mod inputs;
pub mod vault;

use std::cell::RefCell;
use std::rc::Rc;

use fs_mcp::canary::CanaryTable;

pub use drive::{Prog, drive, plant, tasks, vault_prog};
pub use find::Findings;

/// The canaries of a run: one in every place of every tool where a secret would sit, one in the text of an error of the
/// program, one in the key of the store (its `Debug` hides it).
pub fn setup(progs: &[Prog]) -> Rc<RefCell<CanaryTable>> {
    let table = Rc::new(RefCell::new(CanaryTable::new()));
    for p in progs {
        plant(p, &table);
    }
    table.borrow_mut().plant("(program error text)");
    table.borrow_mut().plant("store.EncryptionKey.key");
    table
}

/// The branch name for the list.
pub fn branch() -> String {
    std::env::var("FSCANARY_BRANCH").unwrap_or_else(|_| "local".to_owned())
}
