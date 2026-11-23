//! Gemeinsame Basistypen: `pub use` aus `fs-types` (Tag v0.0.22, fs-interfaces), kein Stand-in mehr (Typ-Tausch,
//! Entscheidung 21, 2026-11-16). `Place` (Weiterblätter-Stelle) heißt dort `PageCursor` (`Place` ist dort der
//! `geo:`-Punkt); hier unter dem alten Namen. Der Fehler des Themas steht in `errors.rs`.
//!
//! Pattern: Value Object (Basistypen aus fs-types; Eintritt nur über `parse`/Konstruktoren, eine Quelle der Wahrheit).

pub use fs_types::PageCursor as Place;
pub use fs_types::{
    Attendee, BasicValue, Capabilities, ContactAddress, Date, FileRef, LocalDateTime, MediaType, ObjectId, Page, PersonArt, PersonRef,
    Problem, Reference, Revision, TypeProblem, Utc, Zone, ZoneKind,
};
