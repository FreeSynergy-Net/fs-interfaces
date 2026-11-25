//! What a module delivers to the wrapper.
//!
//! Pattern: Template Method (steps) - the module delivers only reading, changing, taking back and its rules; the order
//! check -> change -> record -> report stands ONCE in `fs_rpc::Guarded<M>`.

use super::{
    Applied, Blob, ChangePage, ChangeRef, ChangesQuery, Context, Follow, Invocation, Page,
    PageQuery, Refusal, RightsRules, StepRef,
};

/// Pattern: Observer - a callback and no stream type, so this crate gets no new dependency (decision S4-1).
/// Where the module pushes every change (for `watch`); the wrapper filters per subscriber.
/// Not `Send`/`Sync`: it runs in the `LocalSet` of the wrapper (capnp-rpc), never on another thread.
/// The sink is called SYNCHRONOUSLY from inside the change, so it must be quick and must not block. The module calls it
/// without holding any of its own locks (otherwise a sink that calls back into the module could deadlock); the sink itself
/// must not call back into the module.
pub type ChangeFeed = Box<dyn Fn(&ChangeRef)>;

/// Pattern: Observer, second event (like `onError` in Reactive Streams) - the module tells the wrapper that its feed lost changes
/// or ended, so the wrapper can end the subscriptions instead of letting them miss changes silently.
/// New kinds may come (append only), so match with a wildcard arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FeedLoss {
    /// The source of the module was too slow: `missed` changes were dropped before they reached the feed.
    Lagged {
        /// How many changes were lost.
        missed: u64,
    },
    /// The source of the module is gone: no change will come any more.
    Closed,
}

/// Pattern: Observer (the `onError` callback) - where the module reports a [`FeedLoss`]; same rules as [`ChangeFeed`] (not `Send`, called
/// synchronously and without holding a lock of the module, the callback does not call back into the module).
pub type FeedBreak = Box<dyn Fn(FeedLoss)>;

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
    ///
    /// # Panics
    /// An implementation that starts its pump with `tokio::task::spawn_local` panics when called outside a `LocalSet`; the wrapper
    /// `fs_rpc::Guarded` calls `feed` in `new`/`with_clock`, so build it inside the `LocalSet` of the module.
    fn feed(&self, sink: ChangeFeed);

    /// Pattern: Observer, second event - register where the module reports that its feed lost changes or ended ([`FeedLoss`]).
    /// Appended (Stufe 4 V1) with a default body that does nothing (Null Object): a module without a broadcast source loses nothing.
    /// The wrapper registers this BEFORE [`feed`](ModuleCore::feed). After `FeedLoss::Closed` the module calls nothing any more.
    fn on_feed_break(&self, _sink: FeedBreak) {}

    /// Pattern: Repository (lookup by key); deviation: it returns only the canonical id, not a record. Resolves the `scope` of a `watch`
    /// or `changes` call (a name or an id, as the caller wrote it) to the fixed id of the scope (a board UUID, not its name), so a
    /// subscription survives a rename. Appended (Stufe 4 V1) with a default body that passes `scope` through (like a module without scopes).
    /// A module with scopes overrides it. Empty `scope` means "all" and stays empty. An unknown scope is `Err(Refusal::unknown_scope(..))`
    /// (404), never passed through and never [`Follow::all`]. The wrapper asks this BEFORE [`follows`](ModuleCore::follows) and
    /// passes the returned id to it and to [`ChangesQuery::scope`]; `page` is not affected (the module checks its scope itself there).
    ///
    /// # Errors
    /// [`Refusal`]: unknown scope (`unknown-scope`, 404).
    async fn resolve_scope(
        &self,
        _cx: &Context,
        _form: &str,
        scope: &str,
    ) -> Result<String, Refusal> {
        Ok(scope.to_owned())
    }

    /// Pattern: Specification - which changes of `form` and `scope` a subscriber (`watch`) or a query (`changes`) sees; the wrapper
    /// stores it per subscriber and filters the feed and the page of `changes` with [`Follow::admits`].
    /// Appended (S4-B1) with a default body, so a module written before keeps its behaviour: the default follows everything.
    /// A module with scopes overrides it, usually with [`Follow::in_scope`]. Empty `form` and `scope` mean "all". `scope` is the id that
    /// [`resolve_scope`](ModuleCore::resolve_scope) returned. Stays synchronous and pure: existence and ids are `resolve_scope`'s job.
    ///
    /// # Errors
    /// [`Refusal`]: unknown form or scope (like `page`).
    #[allow(clippy::result_large_err)] // `Refusal` is the one refusal type of every method of this trait (the async ones are not linted)
    fn follows(&self, _cx: &Context, _form: &str, _scope: &str) -> Result<Follow, Refusal> {
        Ok(Follow::all())
    }
}
