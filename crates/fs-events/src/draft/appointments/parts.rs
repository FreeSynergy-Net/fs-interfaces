//! Aufzählungen und Unterobjekte (je eine Untertabelle, PLAN.md Abschnitt 2); Regel und Erinnerung aus fs-jscalendar.
//!
//! Pattern: Value Object (Unterobjekte und Aufzählungen, je eine Untertabelle).

use super::{Attendee, FileRef, MediaType, Reference, Utc, Zone};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub enum Status { Confirmed, Tentative, Cancelled }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub enum FreeBusy { Busy, Free }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub enum Privacy { Public, Private, Secret }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub enum TextType { Plain, Html }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub enum ParticipationStatus { NeedsAction, Accepted, Declined, Tentative, Delegated }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub enum Answer { Accept, Tentative, Decline }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub enum Relation { First, Next, Child, Parent }

// Wiederholungsregel (RFC 8984 `RecurrenceRule`), Wochentag, Häufigkeit, Überspringen, `NDay`, `ByMonth`, `RuleEnd`
// kommen aus fs-jscalendar (Typ-Tausch; formgleich mit dem früheren Stand-in, geteilt mit fs-tasks). Im Termin steht je
// Regel ein `Recurrence` wie in tasks_v1: `Rule` (in der Teilmenge, die der Anbieter rechnet) oder `Original` (der empfangene
// RRULE-Text Byte für Byte, ADR 0030): so bleibt eine fremde Regel auch nach einer Änderung an anderen Feldern im Export
// erhalten und die Serie wird nie still zum Einzeltermin. Eine `Original`-Regel zeigt nur das erste Vorkommen ("Serie nur
// teilweise darstellbar"); unverändert zurückgeben ist erlaubt, sie ändern oder eine neue `Original` anlegen ist
// `Unsupported` (PLAN.md §11, Teil 2).
pub use fs_jscalendar::recurrence::{ByMonth, Frequency, NDay, Recurrence, RecurrenceRule, RuleEnd, Skip, Weekday};

/// Wohin eine Einladung oder Antwort geht (`sendTo` / `replyTo`): Methode (`imip`, `other`) und URI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel { pub method: String, pub uri: String }

/// Rollen eines Teilnehmers (`roles`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Roles { pub owner: bool, pub attendee: bool, pub optional: bool, pub informational: bool, pub chair: bool, pub contact: bool }

// Wer teilnimmt (`Attendee`, aus fs-types): Mensch, KI oder Gruppe (`Actor(PersonRef)`), Ressource (Gerät, Raum als
// Ressource) oder Ort, beide Verweise auf fs-resources (ADR 0087). Ein Raum ist kein Akteur (ADR 0076 Regel 1). Der Export
// leitet die JSCalendar-`kind` ab: Actor+Person/Profile = individual, Group/Organization = group, Resource, Location.

/// Teilnehmer. Kein Name-Feld: der Name kommt aus `who` (ADR 0085).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Participant {
    /// Schlüssel im Termin (`JSCalendar` `Id`).
    pub key: String,
    pub who: Attendee,
    pub roles: Roles,
    pub status: ParticipationStatus,
    pub expect_reply: bool,
    pub comment: Option<String>,
    pub send_to: Vec<Channel>,
    pub invited_by: Option<String>,
    pub delegated_to: Vec<String>,
    pub delegated_from: Vec<String>,
    pub schedule_sequence: u32,
    pub schedule_updated: Option<Utc>,
}

// Erinnerung (`alerts`): `Alert`, `Trigger`, `AlertAction`, `RelativeTo` aus fs-jscalendar (Typ-Tausch, formgleich).
// Gespeichert wird nur die Erinnerung; das Auslösen (Meldung, Mail, Nachricht) ist Sache des Programms (Frage 24).
pub use fs_jscalendar::alert::{Alert, AlertAction, RelativeTo, Trigger};

/// Ort (Geografie). `place`: Verweis auf fs-maps (`places`, Strang D E17), sonst nur Text/Koordinaten. Ein
/// buchbarer Raum ist kein Ort, sondern `Attendee::Location`/`Resource` (fs-resources).
#[derive(Clone, Debug, PartialEq)]
pub struct Location {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub types: Vec<String>,
    pub coordinates: Option<(f64, f64)>,
    pub zone: Option<Zone>,
    pub relative_to: Option<RelativeTo>,
    pub place: Option<Reference>,
}

/// Virtueller Ort (Videoanruf).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualLocation { pub key: String, pub name: String, pub uri: String, pub description: Option<String>, pub features: Vec<String> }

/// Verweis (Anhang, URL). Eine Datei bei uns ist `file`, eine fremde Adresse `href`; genau eins von beiden.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link { pub key: String, pub href: Option<String>, pub file: Option<FileRef>, pub content_type: Option<MediaType>, pub size: Option<u64>, pub rel: Option<String>, pub title: Option<String> }

/// Beziehung zu einem anderen Termin per `uid` (z. B. Serie geteilt: `First`/`Next`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Related { pub uid: String, pub relation: Relation }

/// Übersetzung eines Textfelds (`localizations`, nur Textfelder).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Localization { pub language: String, pub field: TextField, pub text: String }
/// Übersetzbare Textfelder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextField { Title, Description, LocationName(String), LocationDescription(String) }
