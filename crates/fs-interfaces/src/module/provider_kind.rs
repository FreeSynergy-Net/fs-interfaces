//! Whether a module is a real module or a source of sample data. Pattern: Value Object (a closed-but-growing kind, read from the text of `hello`).
//!
//! The wire stays text: `hello` carries `provider :Text`, and the contract comment says `provider = "fake"` means sample data. There is no
//! capnp enum on purpose: a new enum field would read `@0` from an old service and show sample data as real. Deviation: none.

/// The `provider` a service says in `hello` when its records are sample data and not a real module's.
pub const SAMPLE_PROVIDER: &str = "fake";

/// What kind of provider a `hello` names. `#[non_exhaustive]`: a host matches `Module => real, _ => "sample data"`, so an unknown kind is never shown as real.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProviderKind {
    /// A real module (any provider id except [`SAMPLE_PROVIDER`]).
    Module,
    /// Sample data (`provider == "fake"`): shown as such, never as real.
    Sample,
}

impl ProviderKind {
    /// The kind a provider text names: exactly [`SAMPLE_PROVIDER`] is `Sample`, everything else `Module`.
    #[must_use]
    pub fn of(provider: &str) -> Self {
        if provider == SAMPLE_PROVIDER {
            Self::Sample
        } else {
            Self::Module
        }
    }
}
