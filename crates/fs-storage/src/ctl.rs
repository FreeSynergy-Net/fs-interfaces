//! `Ctl` (time limit, cancel), `Blob` (range reads) and `ByteSource` (chunked write body) of `storage`.
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

/// The most bytes one `Blob::read` answers (8 MiB): the contract never forces a whole file into one buffer, and a caller
/// cannot make a provider build a buffer of gigabytes with one call.
pub const MAX_READ: u64 = 8 * 1024 * 1024;

/// The bytes of an opened file; the seam to streaming is `read` (ADR 0070 risk 5).
#[async_trait]
pub trait Blob: Send + Sync {
    /// The entry that was opened.
    fn entry(&self) -> &Entry;
    /// Read up to `length` bytes from `offset`. A read may return FEWER bytes than asked (a short read): at most
    /// [`MAX_READ`], and at least one byte while `offset` is before the end; an empty answer means the end or behind it,
    /// never "try again". A caller that needs more reads again from where it stopped. A provider never answers more than
    /// `length`. The provider checks the sha256 of the file against `Entry.file_ref` while reading and answers
    /// `BackendFailed` on a mismatch (it does not hand out bytes that differ from their reference).
    async fn read(&self, offset: u64, length: u64) -> Result<Vec<u8>>;
}

/// The body of a `write`, read in chunks (never one whole buffer in the contract). An empty chunk is the end.
#[async_trait]
pub trait ByteSource: Send {
    /// The next chunk of at most `max` bytes; empty = end.
    async fn next(&mut self, max: usize) -> Result<Vec<u8>>;
}

/// A body held in memory (tests, small writes).
#[derive(Clone, Debug, Default)]
pub struct MemSource {
    data: Vec<u8>,
    pos: usize,
}

impl From<Vec<u8>> for MemSource {
    fn from(data: Vec<u8>) -> Self {
        Self { data, pos: 0 }
    }
}

#[async_trait]
impl ByteSource for MemSource {
    async fn next(&mut self, max: usize) -> Result<Vec<u8>> {
        let end = self.pos.saturating_add(max).min(self.data.len());
        let out = self.data[self.pos..end].to_vec();
        self.pos = end;
        Ok(out)
    }
}
