//! `FailingGroups` and `SlowGroups`: the loading, error, partly broken and disconnected states of the frontends.
//!
//! Pattern: Decorator over `Groups` + `Mirroring` + `GroupMembers`; the failure and the wait are configured per
//! method or per group. The wait is virtual: it moves the clock of the fake (`Clock`), nobody sleeps for real, so a
//! test of "loading for 3 s" is instant and exact.

use crate::decorate::{Decorated, Gate, Op, Provider};
use crate::fake::FakeGroups;
use crate::groups::shared::{ObjectId, Utc};
use crate::groups::{Error, Result, Right, Unreachable};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

/// A clock a slow provider can move forward.
pub trait Clock: Send + Sync {
    /// Move the clock by `ms` milliseconds.
    fn advance(&self, ms: i64);
}

impl Clock for FakeGroups {
    fn advance(&self, ms: i64) {
        self.set_now(Utc::from_millis(self.now().millis() + ms));
    }
}

fn locked<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The error of a service that is gone: `Unavailable(Server)`.
#[must_use]
pub fn unavailable() -> Error {
    Error::Unavailable(Unreachable::Server)
}

/// The error of a refused method: `Forbidden` on `group` for `Manage`.
#[must_use]
pub fn forbidden(group: ObjectId) -> Error {
    Error::Forbidden {
        group,
        right: Right::Manage,
    }
}

/// The gate of [`FailingGroups`].
#[derive(Default)]
pub struct FailGate {
    all: Mutex<Option<Error>>,
    ops: Mutex<HashMap<Op, Error>>,
    groups: Mutex<HashMap<ObjectId, Error>>,
}

impl Gate for FailGate {
    fn pass(&self, op: Op, groups: &[ObjectId]) -> Result<()> {
        if let Some(e) = locked(&self.ops).get(&op) {
            return Err(e.clone());
        }
        if let Some(e) = groups
            .iter()
            .find_map(|g| locked(&self.groups).get(g).cloned())
        {
            return Err(e);
        }
        locked(&self.all).clone().map_or(Ok(()), Err)
    }
}

/// A provider that fails: every method, chosen methods, or every call that names a chosen group.
pub type FailingGroups = Decorated<FailGate>;

impl FailingGroups {
    /// Wrap `inner`; nothing fails yet.
    #[must_use]
    pub fn new(inner: Arc<dyn Provider>) -> Self {
        Self {
            inner,
            gate: FailGate::default(),
        }
    }

    /// Every method answers `error` (error state; disconnected).
    #[must_use]
    pub fn failing_all(self, error: Error) -> Self {
        *locked(&self.gate.all) = Some(error);
        self
    }

    /// The method `op` answers `error`.
    #[must_use]
    pub fn failing(self, op: Op, error: Error) -> Self {
        locked(&self.gate.ops).insert(op, error);
        self
    }

    /// Every call that names `group` by id answers `error` (one group is broken; the others work).
    #[must_use]
    pub fn failing_group(self, group: ObjectId, error: Error) -> Self {
        locked(&self.gate.groups).insert(group, error);
        self
    }

    /// Everything works again (the service came back).
    pub fn heal(&self) {
        *locked(&self.gate.all) = None;
        locked(&self.gate.ops).clear();
        locked(&self.gate.groups).clear();
    }
}

/// The gate of [`SlowGroups`].
pub struct SlowGate {
    clock: Arc<dyn Clock>,
    every: i64,
    ops: Mutex<HashMap<Op, i64>>,
    waited: AtomicI64,
}

impl Gate for SlowGate {
    fn pass(&self, op: Op, _groups: &[ObjectId]) -> Result<()> {
        let ms = locked(&self.ops).get(&op).copied().unwrap_or(self.every);
        self.clock.advance(ms);
        self.waited.fetch_add(ms, Ordering::Relaxed);
        Ok(())
    }
}

/// A provider that takes time: every method `every_ms` of the fake's clock, chosen methods another time (loading).
pub type SlowGroups = Decorated<SlowGate>;

impl SlowGroups {
    /// Wrap `inner`; each call moves `clock` by `every_ms` (the clock is usually the `FakeGroups` inside).
    #[must_use]
    pub fn new(inner: Arc<dyn Provider>, clock: Arc<dyn Clock>, every_ms: i64) -> Self {
        Self {
            inner,
            gate: SlowGate {
                clock,
                every: every_ms,
                ops: Mutex::new(HashMap::new()),
                waited: AtomicI64::new(0),
            },
        }
    }

    /// A slow `FakeGroups`: its own clock is the one that moves.
    #[must_use]
    pub fn of_fake(fake: Arc<FakeGroups>, every_ms: i64) -> Self {
        Self::new(fake.clone(), fake, every_ms)
    }

    /// The method `op` takes `ms` instead of the default.
    #[must_use]
    pub fn taking(self, op: Op, ms: i64) -> Self {
        locked(&self.gate.ops).insert(op, ms);
        self
    }

    /// All the time the calls have taken so far (ms of the fake's clock).
    #[must_use]
    pub fn waited_ms(&self) -> i64 {
        self.gate.waited.load(Ordering::Relaxed)
    }
}
