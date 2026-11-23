//! Der Trait `Appointments` und seine Fähigkeitskennungen.
//!
//! Pattern: Repository (Trait) + Port: jede Methode ist Pflicht, kein Standardverhalten; was ein Anbieter nicht kann,
//! sagt er vorher in `capabilities` und beim Aufruf ausdrücklich mit [`not_possible`] (`Unsupported`).

use super::{
    Answer, Appointment, AppointmentDraft, AppointmentPatch, AppointmentQuery, AppointmentSummary, Attendee,
    Availability, CalendarDeleted, CalendarInfo, CalendarPatch, Capabilities, Change, Deleted, Error, HistoryEntry, Invitee,
    ObjectId, Page, PersonRef, Place, Reference, Refresh, Refreshed, Result, Revision, SeriesScope, Trash, Utc, When,
    Window, Zone,
};
use async_trait::async_trait;

/// Das ausdrückliche "nicht möglich" eines Anbieters, der eine Fähigkeit nicht hat (er ruft es selbst auf; der
/// Trait hat keine Standardmethoden, damit kein Anbieter eine Methode still vergisst).
///
/// # Errors
/// Immer `Unsupported` mit der Fähigkeitskennung.
pub fn not_possible<T>(capability: &str) -> Result<T> {
    Err(Error::Unsupported { capability: capability.to_owned(), reason: "this provider does not offer it".to_owned() })
}

/// Fähigkeitskennungen dieses Themas (Wortlaut für `capabilities`, Manifest und `events.capnp` v1).
pub const CAPABILITIES: [&str; 18] = [
    "calendar.read", "calendar.create", "calendar.update", "calendar.delete", "calendar.restore", "calendar.subscribe",
    "calendar.refresh", "event.read", "event.create", "event.update", "event.delete", "event.restore", "event.search",
    "invite.create", "invite.answer", "event.history", "event.freebusy", "trash.purge",
];
/// Termine: Kalender, Termine anlegen/ändern/löschen, Serien, Einladungen, Erinnerungen, Zeitfenster.
///
/// Ziel v1 (ADR 0045). Erster Anbieter: lokal über fs-objects v5 (`LocalAppointments`). Weitere Anbieter
/// (`CalDAV`, Google, Outlook) implementieren DENSELBEN Trait; was sie nicht können, ist `Unsupported`.
#[async_trait]
pub trait Appointments: Send + Sync {
    /// Was dieser Anbieter kann, je Kennung aus [`CAPABILITIES`].
    fn capabilities(&self) -> Capabilities;

    /// `calendar.read`: alle Kalender, die die fragende Person sehen darf; `trash` wählt lebende, gelöschte oder beide
    /// (`CalendarInfo.trash`; `Only` = Papierkorb-Ansicht).
    async fn calendars(&self, trash: Trash) -> Result<Vec<CalendarInfo>>;
    /// `calendar.create`: eigener Kalender (`CalendarSource::Own`). `owner = None`: die fragende Person; sonst eine
    /// Person, Gruppe oder Organisation, für die sie das Recht hat (fs-access, sonst `Forbidden`); KI als Eigentümerin:
    /// `Forbidden { calendar.create }` (wie `tasks_v1`), `Profile`: `Invalid`.
    async fn create_calendar(&self, name: &str, color: Option<&str>, default_zone: Option<Zone>, owner: Option<PersonRef>) -> Result<CalendarInfo>;
    /// `calendar.update`: Name, Farbe, Vorgabe-Zone, Eigentümer, bei einem Abo Adresse und Abrufregel; Vergleich-und-Setzen.
    ///
    /// Kalender im Papierkorb (gilt für `update_calendar`, `delete_calendar`, `refresh_calendar`): `NotFound(id)`, wie
    /// eine gelöschte Liste in `tasks_v1`; nichts geändert, kein Abruf. Nur `restore_calendar` und `purge` erreichen ihn.
    async fn update_calendar(&self, id: ObjectId, expected: Revision, change: CalendarPatch) -> Result<CalendarInfo>;
    /// `calendar.delete`: Kalender samt seinen Terminen in den Papierkorb (eine Frist für alle, Verlauf geht mit). Auch
    /// der letzte eigene Kalender (das Programm legt sofort einen neuen an).
    async fn delete_calendar(&self, id: ObjectId, expected: Revision) -> Result<CalendarDeleted>;
    /// `calendar.restore`: Kalender samt den Terminen zurück, die mit ihm gelöscht wurden (einzeln vorher gelöschte
    /// bleiben im Papierkorb mit ihrer eigenen Frist). Kein `expected` (im Papierkorb ändert niemand); nach Ablauf der
    /// Frist bzw. nach `purge`: `NotFound`.
    async fn restore_calendar(&self, id: ObjectId) -> Result<CalendarInfo>;
    /// `calendar.subscribe`: Abo-Kalender (`CalendarSource::Subscribed`) anlegen; Adresse geprüft wie bei
    /// [`super::Subscription`] (sonst `Invalid` am Feld `url`); ruft NICHT ab (`FetchState::Never`).
    async fn subscribe_calendar(&self, name: &str, color: Option<&str>, url: &Reference, refresh: Refresh) -> Result<CalendarInfo>;
    /// `calendar.refresh`: Abo bzw. Konto-Sammlung jetzt abrufen/abgleichen (Klick). Ein gescheiterter Abruf ist kein
    /// `Err`, sondern `FetchState::Failed` im Ergebnis; `Err` nur, wenn es weder Abo noch Konto ist (`Invalid`), die
    /// Adresse inzwischen in ein gesperrtes Netz zeigt (`Invalid` am Feld `url`) oder der Speicher fehlt.
    async fn refresh_calendar(&self, id: ObjectId) -> Result<Refreshed>;

    /// `event.read`; ein gelöschter Termin ergibt `None` (sichtbar nur über `list` mit `Trash::Include`/`Only`).
    async fn get(&self, id: ObjectId) -> Result<Option<Appointment>>;
    /// `event.read` über die `uid`.
    async fn get_by_uid(&self, uid: &str) -> Result<Option<Appointment>>;
    /// `event.read`: Termine (Serien einmal), nach erstem Start sortiert, seitenweise.
    async fn list(&self, query: &AppointmentQuery, after: Option<&Place>, size: usize) -> Result<Page<AppointmentSummary>>;
    /// `event.read`: Vorkommen, die `[from, to)` überlappen, Serien aufgelöst (in der Zone des Termins
    /// gerechnet, ADR 0085), aus den gewählten Kalendern (leer = alle). Beschädigtes kostet nur sich.
    /// `view_zone`: Zone des Betrachters; legt `Instance.start/end` für `AllDay` und `Floating` fest. Eine
    /// eigene Zone (`ZoneKind::Custom`) gilt nur mit ihrem Objekt: als `view_zone` ist sie `Invalid` am Feld
    /// `view_zone` (Code der Zonenprüfung, siehe `Error`), ebenso eine IANA-Zone, die die tz-Datenbank nicht kennt.
    async fn window(&self, from: Utc, to: Utc, view_zone: &Zone, calendars: &[ObjectId]) -> Result<Window>;
    /// `event.search`: Volltext in Titel, Beschreibung, Ort.
    async fn search(&self, text: &str, after: Option<&Place>, size: usize) -> Result<Page<AppointmentSummary>>;

    /// `event.create`: prüfen und speichern. Abgelehnt, wenn die `uid` schon existiert; `ReadOnly` in einem Abo.
    /// Für ALLE schreibenden Termin-Methoden (create, update, delete, invite, respond) gilt: liegt der Kalender im
    /// Papierkorb, ist es `NotFound(<Kalender-Id>)` (wie neue Aufgaben in einer gelöschten Liste, `tasks_v1`), nichts
    /// wird geschrieben; `restore` eines Termins darin ist `Invalid` am Feld `calendar` (siehe dort).
    async fn create(&self, draft: AppointmentDraft) -> Result<Appointment>;
    /// `event.update`, Vergleich-und-Setzen gegen `expected`; `scope` bei Serien (sonst `Whole`).
    /// Verschieben (A8), Erinnerung (A6), Zone (A7), ganztägig (A3) sind Felder dieser Änderung.
    async fn update(&self, id: ObjectId, expected: Revision, change: AppointmentPatch, scope: SeriesScope) -> Result<Appointment>;
    /// `event.delete`: in den Papierkorb; `This` legt eine Ausnahme `Excluded` an, `ThisAndFollowing`
    /// kürzt die Serie (`RuleEnd::Until`).
    async fn delete(&self, id: ObjectId, expected: Revision, scope: SeriesScope) -> Result<Deleted>;
    /// `event.restore`: Löschen rückgängig machen, solange `restorable` gilt. `scope` ist derselbe wie beim Löschen
    /// (`This` nimmt die Ausnahme `Excluded` zurück, `ThisAndFollowing` den gekürzten Serienschluss, `Whole` holt den
    /// Termin aus dem Papierkorb); so ist Teil-Löschen umkehrbar. Liegt sein Kalender im Papierkorb: `Invalid`
    /// (`calendar`), zuerst `restore_calendar`. Ein Abo-Termin: `ReadOnly` (die Quelle entscheidet beim Abruf).
    async fn restore(&self, id: ObjectId, scope: SeriesScope) -> Result<Appointment>;
    /// `trash.purge`: einen Termin oder Kalender (samt Terminen) im Papierkorb endgültig löschen, mit Verlauf
    /// (ADR 0073 "wirklich löschen"; der Weg bei `Restorable::Manual`). Nicht im Papierkorb: `Invalid`; aufbewahrt
    /// (`Restorable::Held`, auch ein einzelner Termin eines Kalenders): `Forbidden { capability: "trash.purge" }`,
    /// nichts wird gelöscht. Abo-Termine im Papierkorb dürfen endgültig gelöscht werden.
    async fn purge(&self, id: ObjectId) -> Result<()>;

    /// `invite.create`: Personen einladen (Teilnehmer + Protokoll in einem Schritt). Versand (iMIP) ist
    /// ein `Exchange`-Anbieter, nicht dieser Trait.
    async fn invite(&self, id: ObjectId, expected: Revision, invitees: &[Invitee], scope: SeriesScope) -> Result<Appointment>;
    /// `invite.answer`: als `who` antworten (schreibt nur dessen Teilnehmerzeile), Vergleich-und-Setzen gegen
    /// `expected`; `proposal` = neuer Zeitpunkt vorschlagen (Dossier A11). Wer handelt, ist Eigenschaft des
    /// Anbieters (die fragende Person), nicht dieses Aufrufs.
    #[allow(clippy::too_many_arguments)]
    async fn respond(&self, id: ObjectId, expected: Revision, who: &PersonRef, answer: Answer, comment: Option<&str>, proposal: Option<When>, scope: SeriesScope) -> Result<Appointment>;
    /// `event.freebusy`: Verfügbarkeit von Teilnehmern (Personen, Räume, Ressourcen) im Fenster, je Teilnehmer genau
    /// ein Eintrag in der Reihenfolge von `who`: belegt (ohne Titel), nicht freigegeben oder unbekannt, nie geraten.
    /// Die Freigabe ist eine Einstellung der Person gegenüber der FRAGENDEN Person (fs-access), nie je Umfrage; ohne
    /// Freigabe immer `NotShared`, gleich ob es hier Kalender gibt (siehe [`super::FreeBusyAnswer`]). Die Fensterlänge
    /// begrenzt der Anbieter (zu lang: `Invalid` am Feld `to`), sonst wäre ein Jahres-Scan möglich. Fehler gibt es nur
    /// für den ganzen Aufruf (`Invalid` Fenster/Zone, `Forbidden { event.freebusy }`), nie `NotFound` je Person (auch
    /// nicht für eine KI).
    /// `view_zone`: Zone des Betrachters; legt die Lage von `AllDay` und `Floating` fest (wie bei `window`).
    /// Erweiterungspunkt Terminfindung (siehe `mod.rs`): die Suite verbraucht genau diese Methode.
    async fn free_busy(&self, who: &[Attendee], from: Utc, to: Utc, view_zone: &Zone) -> Result<Vec<Availability>>;

    /// `event.history`: typisiertes Protokoll eines Termins oder Kalenders, älteste zuerst.
    async fn history(&self, id: ObjectId, after: Option<&Place>, size: usize) -> Result<Page<HistoryEntry>>;
    /// `event.read`: Änderungen seit `since` (`None` = ab jetzt), für Beobachter (Raster neu laden).
    async fn changes(&self, since: Option<&Place>, size: usize) -> Result<Page<Change>>;
}
