//! Gemeinsame Basistypen: Wiederverwendung von `fs-types` (>= 0.0.22), kein Stand-in mehr. `Place` (Weiterblätter-
//! Stelle) heißt dort `PageCursor` (`Place` ist dort der `geo:`-Punkt); hier unter dem alten Namen.
//!
//! Pattern: Value Object (Basistypen aus fs-types; Eintritt nur über `parse`/Konstruktoren).

pub use fs_types::PageCursor as Place;
pub use fs_types::{
    BasicValue, ObjectId, Page, PersonArt, PersonRef, Reference, Revision, TypeProblem, Utc,
};
