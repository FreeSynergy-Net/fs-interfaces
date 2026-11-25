//! What a module delivers to the wrapper.
//!
//! Pattern: Template Method (steps) - the module delivers only reading, changing, taking back and its rules; the order
//! check -> change -> record -> report stands ONCE in `fs_rpc::Guarded<M>`.

use super::{
    Applied, Blob, ChangePage, ChangeRef, ChangesQuery, Context, Invocation, Page, PageQuery,
    Refusal, RightsRules, StepRef,
};

/// Pattern: Observer - a callback and no stream type, so this crate gets no new dependency (decision S4-1).
/// Where the module pushes every change (for `watch`); the wrapper filters per subscriber.
/// Not `Send`/`Sync`: it runs in the `LocalSet` of the wrapper (capnp-rpc), never on another thread.
/// The sink is called SYNCHRONOUSLY from inside the change, so it must be quick and must not block. The module calls it
/// without holding any of its own locks (otherwise a sink that calls back into the module could deadlock); the sink itself
/// must not call back into the module.
pub type ChangeFeed = Box<dyn Fn(&ChangeRef)>;

/// Pattern: Template Method (steps) - the module delivers only reading, changing, taking back and its rules; the order
/// check -> change -> record -> report stands once in the wrapper `fs_rpc::Guarded<M>`.
/// Deviation: no default flow in the trait (overridable, so the check could be skipped), a generic wrapper instead; not `dyn`-capable
/// (async methods, the wrapper is generic).
///
/// A module never checks rights in these methods: [`rules`](ModuleCore::rules) are asked by the wrapper BEFORE `apply`/`revert`.
/// Condition: a module runs in the `LocalSet` of the wrapper (capnp-rpc, `spawn_local`); its futures are not `Send` and
/// it must not `tokio::spawn` its own work that needs `&self`.
/// `cx` is always the context of the connection (see [`Context`]); a module keeps no "current user".
#[allow(async_fn_in_trait)]
pub trait ModuleCore: 'static {
    /// The own `FormSource` level of the module (a number, not the constant of the contract).
    fn level(&self) -> u32;

    /// The id of the module as provider (`AppStream` id).
    fn provider(&self) -> &str;

    /// The rules of this module.
    fn rules(&self) -> &dyn RightsRules;

    /// One page of records of a form.
    ///
    /// # Errors
    /// [`Refusal`]: unknown form or scope, bad cursor.
    async fn page(&self, cx: &Context, query: PageQuery) -> Result<Page, Refusal>;

    /// The bytes of a binary field.
    ///
    /// # Errors
    /// [`Refusal`]: unsupported, or no such record or field.
    async fn blob(&self, cx: &Context, record: &str, field: &str) -> Result<Blob, Refusal>;

    /// Run a command (writes through fs-objects `write_by`, which records the change in the same transaction).
    ///
    /// # Errors
    /// [`Refusal`]: bad arguments, conflict (`expect` no longer true), unknown command.
    async fn apply(&self, cx: &Context, command: Invocation) -> Result<Applied, Refusal>;

    /// Take a change back (the whole unit).
    ///
    /// # Errors
    /// [`Refusal`]: `conflict`, `not-revertible`, `gone`.
    async fn revert(&self, cx: &Context, change: &str, step: StepRef) -> Result<Applied, Refusal>;

    /// The changes of the asking program, or of one run.
    ///
    /// # Errors
    /// [`Refusal`]: bad cursor.
    async fn changes(&self, cx: &Context, query: ChangesQuery) -> Result<ChangePage, Refusal>;

    /// Register `sink` for every change of this module; the wrapper filters per subscriber.
    fn feed(&self, sink: ChangeFeed);
}
