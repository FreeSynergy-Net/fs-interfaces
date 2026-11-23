//! Thema `events`, Ziel v1 (Sprung von "events 0.1 (draft)"). Eigentümer: fs-calendars (ADR 0050). Trait `Appointments`.
//!
//! Ordner (ADR 0043, jede Datei <= 300 Zeilen), ARBEITS-FIXPUNKT per `tools/hash-ordner.sh appointments_v1` (SHA-256
//! über sortierte relative Pfade und Inhalte, `LC_ALL=C`). Vorgänger-Fixpunkt `2b7fc98b…19b8` (Stand-in-Typen). Dieser
//! Stand = Typ-Tausch (Entscheidung 21, 2026-11-16: vor dem ersten Release IN v1 geändert, kein Sprung auf v2) plus
//! Kalender verwalten, Papierkorb-Frist, Abo- und Konto-Kalender, Nutzerantworten 22-25 (Runde 2 nach Opus-Review),
//! Nachlauf K1-K8 (Freigabe, Konto-Adresse, SSRF, KI-Eigentum, Papierkorb wie `tasks_v1`, Personen über fs-people).
//! Feature-Freeze: offen (Nutzer).
//! Dateien: `mod.rs` (dieser Kopf), `shared.rs` (`pub use` aus fs-types), `errors.rs` (Fehler des Themas), `time.rs`
//! (`Span`, `When`; `Origin` aus fs-jscalendar), `parts.rs` (Unterobjekte; Regel und Erinnerung aus fs-jscalendar),
//! `calendar.rs` (Kalender, Abo, Papierkorb-Frist), `appointment.rs`, `query.rs`, `api.rs` (Trait `Appointments`),
//! `exchange.rs` (Grenze `Exchange`). Personen: `PersonDirectory` aus `person_directory_v1` (fs-people), verbraucht
//! wie in `tasks_v1` (der Anbieter bekommt ihn im Konstruktor; kein eigener Port, kein Import hier).
//!
//! GEMEINSAME TYPEN (Pins: fs-types = fs-interfaces Tag v0.0.22, fs-jscalendar Tag v0.0.2, der fs-types v0.0.22
//! verwendet; immer per `tag`, nie per `rev` desselben Commits: sonst stehen ZWEI fs-types im Lock und die Typen sind
//! verschieden): `Utc`, `LocalDateTime`, `Date`, `Zone`, `Reference`, `PersonRef`, `Attendee`, `ContactAddress`, `ObjectId`, `Revision`, `MediaType`, `FileRef`, `Page`, `Capabilities`,
//! `Problem` aus fs-types (`Place` = `PageCursor`); `Recurrence`/`RecurrenceRule` samt Teilen, `Alert` samt Teilen, `Origin` aus
//! fs-jscalendar. Ein anderer Name, Konstruktor oder Zugriff dort ist ein Sprung dieses Traits (ADR 0045).
//!
//! Pattern: Repository (Trait) + Port (ADR 0043/0045). Jede Methode ist Pflicht, keine Standardmethode (Lehre aus
//! `groups_v1`/`packages_v1`: nichts versteckt Verhalten); was ein Anbieter nicht kann, sagt `capabilities` vorher,
//! damit kein Frontend einen toten Knopf zeigt, und der Aufruf antwortet ausdrücklich `Unsupported`
//! ([`not_possible`]). Formate (ICS, JSCalendar-JSON, iTIP) stehen NICHT in diesem Trait: sie sind
//! `Exchange`-Anbieter. Konten (`CalDAV`, Google, Outlook) sind eigene Anbieter DIESES Traits in eigenen Crates.
//! `#[non_exhaustive]` nur an Fehler-/Ergebnis-Enums, nie an Structs (der Fake baut sie außerhalb des Crates).
//!
//! Regeln des Vertrags: Wer handelt, ist die fragende Person des Anbieters, nie ein Parameter (außer `respond.who`,
//! geprüft); die Uhr ebenso (Papierkorb-Frist, Abruf). Jedes Schreiben auf Bestehendes ist Vergleich-und-Setzen gegen
//! `expected: Revision`; ausgenommen Anlegen, `restore`, `restore_calendar`, `refresh_calendar`, `purge`.
//! KI (ADR 0056, Nutzerantwort 29 "KI-Helfer nur auf Einladung"): ein KI-Konto ist nie Eigentümer eines Kalenders
//! (`Forbidden { calendar.create | calendar.update }`, Eigentum ist Verwaltung, wie `tasks_v1`; ein `Profile` als
//! Eigentümer ist `Invalid`) und darf nie `purge`, `delete_calendar`, `subscribe_calendar` (`Forbidden`); es handelt
//! nur als eingeladener Helfer einer Person (Teilnehmer bzw. Recht aus fs-access, jederzeit entziehbar). Was es nicht
//! sehen darf (fremde oder sensible Kalender), antwortet wie eine unbekannte Id (`NotFound`), nicht `Forbidden`.
//! Papierkorb (ADR 0073 Nachtrag 2026-11-16): Löschen = Papierkorb mit Frist ([`Restorable`]); die Frist ist eine
//! Pflicht-Einstellung des Programms (Standard 30 Tage, mindestens 1 Tag, "nie automatisch" erlaubt), keine
//! Trait-Sache; eine Aufbewahrung (`keep_until`) geht vor; nach Ablauf geht das Objekt samt Verlauf (Kaskade).
//!
//! Zeiten (ADR 0085): Zeitpunkte UTC (`Utc`, ms); Termine mit lokaler Bedeutung = `LocalDateTime` + `Zone`;
//! ganztägig = `Date` ohne Zone, nie umgerechnet; `When::Floating` ("überall 8 Uhr") ist KEIN Zeitpunkt, sondern eine
//! eigene Art: nie umgerechnet; entsteht nur durch Menschen, durch die eigene Marke (`X-FS-FLOATING` /
//! `freesynergy.net:floating`) oder über den Kanal `Hub`, nie durch eine Zeit ohne Zone allein (Konformitätstest:
//! Datei mit Zeit ohne Zone und ohne Marke ergibt `Timed { zone_assumed: true }`, nie `Floating`). Zonen: EINE Quelle
//! der Wahrheit (Entscheidung 2a): die Form prüft `fs_types::Zone`, ob es eine IANA-Zone gibt, die tz-Datenbank
//! (fs-jscalendar); eine unbekannte Zone ist `Error::Invalid` mit dem Zonen-Code von fs-types, kein eigener Fehler.
//! Menschen (ADR 0085): nur `PersonRef` bzw. `Attendee`, nie ein freier Name. Speicher (ADR 0086): jedes Feld hat
//! eine typisierte Spalte (PLAN.md Abschnitt 2).
//!
//! ERWEITERUNGSPUNKT Terminfindung ("Doodle", pilot-terminfindung/PLAN.md): KEIN Feature dieses Traits. Die
//! Abstimmung ist eine SUITE; Umfrage, Vorschläge und Stimmen gehören fs-forms, Gäste ohne Konto kommen über
//! fs-admission und `propose` von `person_directory_v1` als `PersonRef` (`Profile`). Dieser Trait liefert nur:
//! `free_busy` (`Availability`: belegt / nicht freigegeben / unbekannt; Freigabe = Einstellung der Person gegenüber der
//! fragenden Person, nie je Umfrage; die Suite fragt nur `who = [ich]` in den vorgeschlagenen Fenstern),
//! `create` (Vormerkung `Status::Tentative`) + `invite` im Kalender des Besitzers (Person oder Gruppe), `respond`
//! (auch `proposal`). Umfrage und Termin verweisen über `links` (IRI) aufeinander, kein eigenes Feld.
//!
//! NUTZERANTWORTEN (2026-11-16, Empfehlungen des Opus-Reviews, von der Strategie bestätigt):
//! - 22 Gruppe als Eigentümerin: `owner: PersonRef` (Person/Group/Organization) in `create_calendar` und
//!   `CalendarPatch`; Rechte nur über fs-access (Rechte-Teil-Trait), hier kein eigenes Rechtemodell.
//! - 23 `CalDAV`: `CalendarSource::Account { collection, state }`; Abgleich ist Anbieter-Technik.
//! - 24 Erinnerung Gerät + Mail/Nachricht: PROGRAMM (Einstellung der Person, nicht des geteilten Termins); `Alert`
//!   bleibt wie in RFC 8984.
//! - 25 Abo per Link: aktiv (`CalendarSource::Subscribed`, Regeln an [`Subscription`]); Terminfindung siehe oben.

// Die Formen folgen JSCalendar Feld für Feld (Wahrheitswerte von `Roles` und `Appointment`, `OverrideChange::Changed` =
// `PatchObject`); ein Umbau nur für die Lints wäre ein Formwechsel gegen RFC 8984 und gegen den Fake.
#![allow(clippy::struct_excessive_bools, clippy::large_enum_variant)]

mod api;
mod appointment;
mod calendar;
mod errors;
mod exchange;
mod parts;
mod query;
pub mod shared;
mod time;

pub use api::*;
pub use appointment::*;
pub use calendar::*;
pub use errors::*;
pub use exchange::*;
pub use parts::*;
pub use query::*;
pub use shared::*;
pub use time::*;
