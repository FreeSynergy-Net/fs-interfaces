//! The module side of rights (`FormSource` level 2, Stufe 4 part 1): `Context`, `RightsRules`, `ModuleCore` and their values.
//!
//! Pattern: Template Method (as a wrapper) + Strategy - a module delivers only reading, changing, taking back and its own
//! rules ([`ModuleCore`], [`RightsRules`]); the order check -> change -> record -> report is written ONCE, in the
//! wrapper `fs_rpc::Guarded<M>` (not here: this crate stays free of tokio and fs-rpc). Deviation: no default flow in the
//! trait (in Rust it could be overridden and the check skipped), a generic wrapper instead.
//!
//! Deviation from trait-delta rule 6 ("`#[non_exhaustive]` only on enums"): the structs `Context`, `Claimed`, `Ask` and the
//! value structs of `values` are `#[non_exhaustive]` too. Reason: the contract only grows (append only), and a struct literal
//! outside this crate (above all of `Context`) must not be possible; rule 6 would allow new fields only as a breaking change.
//! (`RightsRules::hidden` has no default method, as rule 6 asks.)
//!
//! The contracts only grow (append only): the types that may get new variants are `#[non_exhaustive]`; the value types
//! of [`values`] are plain data that the wrapper and the modules build. The Cap'n Proto conversion (`wire`) is not part of this step
//! (it needs the level-2 schema words that the stage 3 landing fixes); the values mirror `schema/formsource.capnp`.

mod change;
mod context;
mod core;
mod follow;
mod provider_kind;
mod rules;
mod values;

#[cfg(feature = "fake")]
pub mod fake;

pub use self::change::{ChangeKind, ChangePage, ChangeRef, ChangesQuery, Conflict, FieldClash};
pub use self::context::{Actor, Caller, Claimed, Context};
pub use self::core::{ChangeFeed, FeedBreak, FeedLoss, ModuleCore};
pub use self::follow::Follow;
pub use self::provider_kind::{ProviderKind, SAMPLE_PROVIDER};
pub use self::rules::{Ask, HiddenReason, RightsRules, Verdict};
pub use self::values::{
    Applied, Blob, HiddenField, Invocation, Origin, Page, PageQuery, RecordView, Refusal, StepRef,
    Value,
};
