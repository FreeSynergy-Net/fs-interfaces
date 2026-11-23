//! Termin, Entwurf, Änderung, Ausnahme.
//!
//! Pattern: Value Object (Termin, Entwurf, Änderung als reine Daten).

use super::{
    Alert, Channel, FreeBusy, Link, LocalDateTime, Localization, Location, ObjectId, Origin, Participant, Privacy,
    Recurrence, Related, Revision, Status, TextType, Utc, VirtualLocation, When,
};

/// Der Termin (`JSCalendar` `Event`), vollständig typisiert.
#[derive(Clone, Debug, PartialEq)]
pub struct Appointment {
    pub id: ObjectId,
    pub calendar: ObjectId,
    pub uid: String,
    pub revision: Revision,
    pub sequence: u32,
    pub created: Utc,
    pub updated: Utc,
    pub title: String,
    pub description: String,
    pub description_type: TextType,
    pub locale: Option<String>,
    pub when: When,
    pub status: Status,
    pub free_busy: FreeBusy,
    pub privacy: Privacy,
    pub priority: u8,
    pub color: Option<String>,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub locations: Vec<Location>,
    pub virtual_locations: Vec<VirtualLocation>,
    /// Anhänge und Verweise; auch der Rückverweis auf eine Terminumfrage (fs-forms, IRI), kein eigenes Feld.
    pub links: Vec<Link>,
    pub participants: Vec<Participant>,
    pub reply_to: Vec<Channel>,
    pub alerts: Vec<Alert>,
    pub use_default_alerts: bool,
    pub recurrence: Vec<Recurrence>,
    pub excluded_recurrence: Vec<Recurrence>,
    pub overrides: Vec<Override>,
    pub related: Vec<Related>,
    pub localizations: Vec<Localization>,
    pub hide_attendees: bool,
    pub may_invite_self: bool,
    pub may_invite_others: bool,
    /// Herkunft, wenn importiert (nur lesend; `create`/`update` ignorieren es).
    pub origin: Option<Origin>,
}

/// Neuer Termin. `uid = None`: der Anbieter vergibt eine.
#[derive(Clone, Debug, PartialEq)]
pub struct AppointmentDraft {
    pub calendar: ObjectId,
    pub uid: Option<String>,
    pub title: String,
    pub when: When,
    pub rest: AppointmentPatch,
}

/// Änderung: `Some` = Feld wird gesetzt (Listen ganz ersetzt), `None` = bleibt. Dieselbe Form beschreibt eine
/// Ausnahme einer Serie (`JSCalendar` `PatchObject`, typisiert).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AppointmentPatch {
    pub title: Option<String>,
    pub description: Option<(String, TextType)>,
    pub locale: Option<Option<String>>,
    pub when: Option<When>,
    pub status: Option<Status>,
    pub free_busy: Option<FreeBusy>,
    pub privacy: Option<Privacy>,
    pub priority: Option<u8>,
    pub color: Option<Option<String>>,
    pub keywords: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub locations: Option<Vec<Location>>,
    pub virtual_locations: Option<Vec<VirtualLocation>>,
    pub links: Option<Vec<Link>>,
    pub participants: Option<Vec<Participant>>,
    pub alerts: Option<Vec<Alert>>,
    pub use_default_alerts: Option<bool>,
    pub recurrence: Option<Vec<Recurrence>>,
    pub excluded_recurrence: Option<Vec<Recurrence>>,
    pub localizations: Option<Vec<Localization>>,
}

/// Eine Ausnahme einer Serie, benannt durch die ursprüngliche lokale Startzeit des Vorkommens.
#[derive(Clone, Debug, PartialEq)]
pub struct Override { pub recurrence_id: LocalDateTime, pub change: OverrideChange }
/// Ausnahme: Vorkommen fällt aus, oder es weicht ab. In `Changed` sind `recurrence` und `excluded_recurrence` nicht
/// erlaubt (`Invalid`).
#[derive(Clone, Debug, PartialEq)]
pub enum OverrideChange { Excluded, Changed(AppointmentPatch) }

/// Umfang einer Änderung an einer Serie (Serienfrage; Dossier A9/A10). Für Einzeltermine nur `Whole`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeriesScope {
    /// Der ganze Termin bzw. die ganze Serie.
    Whole,
    /// Nur dieses Vorkommen (wird eine Ausnahme).
    This { recurrence_id: LocalDateTime },
    /// Dieses und alle folgenden: die Serie wird geteilt (neue `uid`, `Related::Next`/`First`).
    ThisAndFollowing { recurrence_id: LocalDateTime },
}
