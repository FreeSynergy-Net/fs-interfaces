//! `Ctl` (time limit, cancel) and `Blob` (range reads) of `storage`.
//!
//! Pattern: Context object (`Ctl`) and Port (`Blob`).

use crate::entry::Entry;
use crate::error::{Error, Result};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Control of a long call: a time limit for the caller to enforce (`call_limited`) and a cancel flag the provider
/// checks between steps.
#[derive(Clone, Debug, Default)]
pub struct Ctl {
    /// Time limit.
    pub deadline: Option<Duration>,
    cancel: Arc<AtomicBool>,
}

impl Ctl {
    /// No limit, not cancelled.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Ask the provider to stop.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// Whether cancel was asked.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    /// For providers: `Err(Cancelled)` when cancel was asked.
    ///
    /// # Errors
    /// `Error::Cancelled`.
    pub fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// The bytes of an opened file; the seam to streaming is `read` (ADR 0070 risk 5).
#[async_trait]
pub trait Blob: Send + Sync {
    /// The entry that was opened.
    fn entry(&self) -> &Entry;
    /// Read `length` bytes from `offset`; shorter at the end, empty behind it.
    async fn read(&self, offset: u64, length: u64) -> Result<Vec<u8>>;
}
