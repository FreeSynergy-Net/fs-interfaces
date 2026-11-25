//! The subscription filter of a module: which changes of a form and scope a subscriber (`watch`) or a query (`changes`) sees.
//!
//! Pattern: Specification - a test "does this change belong to what is followed?" as a value, built by the module and asked by the
//! wrapper. Deviation: the test is a closure (`Rc`, not `Send`: it runs in the `LocalSet` of the wrapper, like [`ChangeFeed`](super::ChangeFeed)),
//! so there is no combinator algebra; the module builds the one test it needs.

use std::fmt;
use std::rc::Rc;

use super::ChangeRef;

/// Pattern: Specification - which changes one subscriber follows. [`Follow::all`] admits every change (the default of
/// [`ModuleCore::follows`](super::ModuleCore::follows)); a module with scopes (boards, folders) builds a narrower one with [`Follow::by`].
#[derive(Clone)]
pub struct Follow {
    test: Option<Rc<dyn Fn(&ChangeRef) -> bool>>,
}

impl Follow {
    /// Admits every change (no filter).
    #[must_use]
    pub fn all() -> Self {
        Self { test: None }
    }

    /// Admits the changes for which `test` is true.
    #[must_use]
    pub fn by(test: impl Fn(&ChangeRef) -> bool + 'static) -> Self {
        Self {
            test: Some(Rc::new(test)),
        }
    }

    /// Whether `change` belongs to what is followed.
    #[must_use]
    pub fn admits(&self, change: &ChangeRef) -> bool {
        self.test.as_ref().is_none_or(|t| t(change))
    }
}

impl Default for Follow {
    fn default() -> Self {
        Self::all()
    }
}

impl fmt::Debug for Follow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = if self.test.is_some() { "by" } else { "all" };
        f.debug_tuple("Follow").field(&kind).finish()
    }
}
