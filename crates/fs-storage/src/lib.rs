//! `fs-storage`: the contract `storage` (ADR 0070, addendum 23.11.2026): one tree of everything a person may reach.
//!
//! Pattern: Facade (one door for the trait, the value types, a fake with a sample tree and the conformance rules).
//!
//! One contract, no `_vN` (ADR 0045 addendum). It REPLACES `schema/files.capnp` (0 consumers, marked as replaced there).
//! `Entry.owner` is a [`PersonRef`] (a person or a group), `Entry.file_ref` is the [`FileRef`] of fs-types (sha256).
//! There is no own rights model: the provider decides per asker (the person on whose behalf it is called); the real
//! module resolves group owners at run time through the group contract (ADR 0095). An AI is never an owner.
//!
//! Reused instead of written here: `cap-std` (path confinement) and `infer` / `mime_guess` (file type) are for the
//! providers in `fs-modules-file`, not for this contract (it has no disk access).

pub mod conformance;
pub mod ctl;
pub mod entry;
pub mod error;
pub mod fake;
pub mod ids;
pub mod provider;

pub use ctl::{Blob, Ctl};
pub use entry::{
    Change, ChangeKind, ChangeSet, Delete, Entry, Kind, OnConflict, PageReq, ProviderState, Root,
    RootKind, Sort, StorageCaps, Token, Usage, WriteOpts,
};
pub use error::{Error, NameProblem, Result};
pub use fake::{FakeLocal, FakeStorage, Fault, Sample, sample};
pub use fs_types::{FileRef, MediaType, PersonArt, PersonRef};
pub use ids::{EntryId, split_path, validate_name};
pub use provider::{Listener, StorageProvider, Subscription};
