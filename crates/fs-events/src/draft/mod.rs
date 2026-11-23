//! The `Appointments` trait draft (pilot `programm-grundgeruest`, plan step 2).
//!
//! `appointments_v1/` is a byte-exact copy of the folder `.scratch/pilot-calendar/appointments_v1/` (folder hash, `tools/hash-ordner.sh`:
//! `069c29c64b1488bdf16f395b9a60f6784aad08a0a02acaa522ced27330e7de98`, working fixpoint after the type swap, 2026-11-16). It is
//! NOT a feature freeze (only the user sets that): the types come from fs-types v0.0.22 and fs-jscalendar v0.0.2 (tags).
//! When the trait moves to `fs-calendars-core`, this module becomes `pub use` of it and nothing else in this crate changes.
//! Do not edit the copy: a different trait is a new plan (fixpoint folder rule).

#[rustfmt::skip]
#[allow(clippy::all, clippy::pedantic, missing_docs)]
pub mod appointments_v1;

pub use appointments_v1::shared::*;
pub use appointments_v1::*;
