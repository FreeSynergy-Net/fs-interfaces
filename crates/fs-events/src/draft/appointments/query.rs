//! Abfragen und Antworten, Protokoll.
//!
//! Pattern: Value Object (Abfrage-, Antwort- und Protokolltypen).

use super::{
    Attendee, Date, FreeBusy, LocalDateTime, ObjectId, ParticipationStatus, PersonRef, Privacy, Reference, Restorable,
    Revision, Roles, SeriesScope, Status, Utc, When, Zone,
};

/// Ein Vorkommen im Zeitfenster (für Raster und Agenda). Zeiten schon für Sortieren/Lage in UTC.
#[derive(Clone, Debug, PartialEq)]
pub struct Instance {
    pub appointment: ObjectId,
    pub calendar: ObjectId,
    pub recurrence_id: Option<LocalDateTime>,
    pub title: String,
    pub when: When,
    pub start: Utc,
    pub end: Utc,
    pub status: Status,
    pub free_busy: FreeBusy,
    pub privacy: Privacy,
    pub overridden: bool,
    pub recurring: bool,
    pub participant_count: u32,
    /// Teilnahmestatus der fragenden Person, wenn sie Teilnehmer ist.
    pub my_status: Option<ParticipationStatus>,
}

/// Was im Fenster nicht beantwortet werden konnte (kostet nur sich selbst).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Damaged { pub appointment: ObjectId, pub recurrence_id: Option<LocalDateTime>, pub reason: String, pub cut: bool }

/// Antwort auf ein Zeitfenster.
#[derive(Clone, Debug, Default)]
pub struct Window { pub instances: Vec<Instance>, pub damaged: Vec<Damaged> }

/// Kurzform für Listen und Suche. `trash`: `None` = nicht gelöscht, `Some` = im Papierkorb mit Frist (nur mit
/// `Trash::Include`/`Only` sichtbar).
#[derive(Clone, Debug, PartialEq)]
pub struct AppointmentSummary { pub id: ObjectId, pub calendar: ObjectId, pub uid: String, pub title: String, pub when: When, pub recurring: bool, pub revision: Revision, pub trash: Option<Restorable> }

/// Filter für `list`.
#[derive(Clone, Debug, Default)]
pub struct AppointmentQuery { pub calendars: Vec<ObjectId>, pub text: Option<String>, pub from: Option<Utc>, pub to: Option<Utc>, pub participant: Option<Attendee>, pub trash: Trash }

/// Papierkorb in einer Abfrage (Termine und Kalender, wie `tasks_v1`). Standard der Ansichten: `Exclude`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Trash {
    /// Nur lebende.
    #[default]
    Exclude,
    /// Lebende und gelöschte gemischt; `trash` unterscheidet sie.
    Include,
    /// Nur gelöschte (die Papierkorb-Ansicht), nach dem Ende ihrer Frist sortiert.
    Only,
}

/// Wen einladen (Menschen, Räume, Ressourcen).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invitee { pub who: Attendee, pub roles: Roles, pub expect_reply: bool }

/// Ergebnis eines Löschens (Papierkorb; `restore` holt zurück, solange `restorable` gilt).
#[derive(Clone, Debug, PartialEq)]
pub struct Deleted { pub id: ObjectId, pub scope: SeriesScope, pub restorable: Restorable }

/// Verfügbarkeit eines Teilnehmers (Person, Raum, Ressource; Dossier A5) im gefragten Fenster.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Availability { pub who: Attendee, pub answer: FreeBusyAnswer }
/// Was über einen Teilnehmer gesagt werden darf. "Frei" ist `Busy` mit leerer Liste; nie geraten.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FreeBusyAnswer {
    /// Belegte Zeiten (ohne Titel), leer = frei.
    Busy(Vec<Busy>),
    /// Die Person gibt ihre Zeiten der fragenden Person nicht frei (Einstellung der Person, fs-access). Immer diese
    /// Antwort ohne Freigabe, gleich ob die Person hier Kalender hat oder nicht: sonst verriete sie das.
    NotShared,
    /// Der Anbieter führt diesen Teilnehmer nicht (Gast-Profil, fremde Ressource); nie für eine Person ohne Freigabe.
    Unknown,
}
/// Eine belegte Zeit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Busy { pub start: Utc, pub end: Utc, pub tentative: bool }

/// Ein typisierter Eintrag im Aktivitätsprotokoll (ADR 0086 Punkt 2).
#[derive(Clone, Debug, PartialEq)]
pub struct HistoryEntry { pub at: Utc, pub actor: PersonRef, pub kind: HistoryKind, pub changes: Vec<FieldChange> }
/// Art der Aktivität (AS2-Name).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HistoryKind { Create, Update, Delete, Undo, Invite, Accept, TentativeAccept, Reject }
/// Feld, alter Wert, neuer Wert.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldChange { pub field: String, pub row_key: Option<String>, pub old: Option<Value>, pub new: Option<Value> }
/// Typisierter Wert im Protokoll. `Object` = Objekt-Id im eigenen Speicher, `Reference` = fremder oder
/// programmübergreifender Verweis. Aufzählungswerte (`when_kind`, `status` …) stehen als `Text` mit dem
/// kanonischen `JSCalendar`- bzw. AS2-Wort.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Value { Int(i64), Real(f64), Text(String), Bool(bool), Utc(Utc), Local(LocalDateTime), Date(Date), Zone(Zone), Object(ObjectId), Reference(Reference), Person(PersonRef), Attendee(Attendee) }

/// Änderung für Beobachter (Raster und Seitenleiste neu laden). `appointment = None`: der Kalender selbst hat sich
/// geändert. Arten: Papierkorb und `purge` sind beide `Delete` (ein zweites `Delete` desselben Objekts = endgültig weg),
/// `restore`/`restore_calendar` = `Undo`; ein Abruf meldet je Termin `Create`/`Update`/`Delete` und für den Kalender
/// (Abrufzustand) ein `Update` mit `appointment = None`.
#[derive(Clone, Debug, PartialEq)]
pub struct Change { pub appointment: Option<ObjectId>, pub calendar: ObjectId, pub kind: HistoryKind, pub at: Utc }
