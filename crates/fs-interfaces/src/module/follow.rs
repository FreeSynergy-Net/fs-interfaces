//! The subscription filter of a module: which changes of a form and scope a subscriber (`watch`) or a query (`changes`) sees.
//!
//! Pattern: Specification - a test "does this change belong to what is followed?" as a value, built by the module and asked by the
//! wrapper. Deviation: the test is a closure (`Rc`, not `Send`: it runs in the `LocalSet` of the wrapper, like [`ChangeFeed`](super::ChangeFeed)),
//! so there is no combinator algebra; the module builds the one test it needs.

use std::fmt;
use std::rc::Rc;

use super::ChangeRef;

type Test = Rc<dyn Fn(&ChangeRef) -> bool>;

/// Pattern: Specification - which changes one subscriber follows. [`Follow::all`] admits every change (the default of
/// [`ModuleCore::follows`](super::ModuleCore::follows)); a module with scopes (boards, folders) builds a narrower one with [`Follow::by`].
#[derive(Clone)]
pub struct Follow {
    test: Option<Test>,
}

impl Follow {
    /// Admits every change (no filter).
    #[must_use]
    pub fn all() -> Self {
        Self { test: None }
    }

    /// Admits the changes for which `test` is true.
    ///
    /// The wrapper calls `test` synchronously when it reports a change (`hub`) and while it filters a page of `changes`; it is a callback
    /// into the module. It must be quick, must not block and must not hold or take a `borrow_mut` of module state.
    #[must_use]
    pub fn by(test: impl Fn(&ChangeRef) -> bool + 'static) -> Self {
        Self {
            test: Some(Rc::new(test)),
        }
    }

    /// Admits the changes that lie in the scope `id` (the fixed id from
    /// [`ModuleCore::resolve_scope`](super::ModuleCore::resolve_scope), see [`ChangeRef::scopes`]). The one specification all modules
    /// use for scopes, so no module writes its own closure. An empty `id` means "all scopes" (like an empty `scope` at `follows`) and
    /// admits every change.
    #[must_use]
    pub fn in_scope(id: &str) -> Self {
        if id.is_empty() {
            return Self::all();
        }
        let id = id.to_owned();
        Self::by(move |c| c.scopes.contains(&id))
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
