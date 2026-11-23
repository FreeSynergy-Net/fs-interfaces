//! Kalender (Sammlung von Terminen): eigene, abonnierte und Konto-Kalender, ändern, Papierkorb.
//!
//! Pattern: Value Object (Kalender, Quelle, Abo-Zustand und Papierkorb-Frist als reine Daten).

use super::{ImportReport, ObjectId, PersonRef, Reference, Revision, Utc, Zone};

/// Ein Kalender. Sichtbarkeit und Farbe in der Seitenleiste sind Einstellung der Person, nicht Daten; `color` hier
/// ist die Vorgabe des Kalenders. `owner`: Person, Gruppe oder Organisation (`PersonArt::Person`/`Group`/
/// `Organization`; Frage 22: JA; `Profile` nie (`Invalid`), ein KI-Konto nie (`Forbidden`, Eigentum ist Verwaltung,
/// ADR 0056, wie `tasks_v1`)); WAS jemand darf, sagt fs-access (Rechte-Teil-Trait), nicht dieses Feld. `trash`: `None` = nicht gelöscht, `Some` = im Papierkorb (nur mit `Trash::Include`/`Only`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarInfo {
    pub id: ObjectId,
    pub name: String,
    pub color: Option<String>,
    pub default_zone: Option<Zone>,
    pub owner: PersonRef,
    pub source: CalendarSource,
    pub trash: Option<Restorable>,
    pub revision: Revision,
}

/// Woher die Termine eines Kalenders kommen.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CalendarSource {
    /// Eigener Kalender: Termine entstehen hier (`create`, `Exchange::import`).
    Own,
    /// Abo eines fremden Kalenders per Link (Frage 25.2: JA, aktiv). SCHREIBGESCHÜTZT: Termine darin schreibt nur der
    /// Abruf (`refresh_calendar`); `create`/`update`/`delete`/`restore`/`invite`/`respond` ergeben `Error::ReadOnly`
    /// (`purge` eines Abo-Termins im Papierkorb ist erlaubt). Jeder Termin trägt `Origin { channel: Subscription,
    /// source: <IRI dieses Kalenders> }`, NIE die Feed-Adresse (sie kann geheim sein, siehe [`Subscription`]).
    Subscribed(Subscription),
    /// Sammlung eines Kontos (`CalDAV`, RFC 4791; Frage 23). `collection`: IRI der Sammlung, `state`: letzter Abgleich.
    /// `collection` ist wie [`Subscription::url`] zu behandeln (Pfade tragen oft Konto-Mail oder Kennung): sichtbar nur
    /// mit `calendar.update` an diesem Kalender (sonst `None`), nie im Verlauf als Klartext, nie in `Origin.source` oder
    /// einem Export. Die Adressregel von [`Subscription`] (SSRF) gilt auch beim Einrichten eines Kontos (Programm).
    /// Der Abgleich (Sync-Token, `ETag` je Termin in `Origin.foreign_etag`) ist Technik des Anbieters, kein Vertrag.
    Account { collection: Option<Reference>, state: FetchState },
}

/// Ein Abo: Quelle, Abrufregel, letzter Abruf und Fehlerzustand.
///
/// Adressregel (Schutz vor SSRF; beim Anlegen, bei jedem Abruf und beim Einrichten eines Kontos), sonst `Invalid` am
/// Feld `url` (bzw. `collection`):
/// - nur `https:` und `webcal:` (= https); Zugangsdaten in der Adresse (`user:pass@`) sind `Invalid`;
/// - geprüft wird die IP, mit der wirklich verbunden wird: auflösen, prüfen, an GENAU diese IP verbinden (kein
///   DNS-Rebinding); jede Umleitung wird neu geprüft und darf nur auf `https:` zeigen (nie zurück auf `http:`);
/// - gesperrt ist jeder Bereich, den die IANA-Register "IPv4/IPv6 Special-Purpose Address" als nicht global
///   erreichbar führen (u. a. `0.0.0.0/8`, `10/8`, `100.64/10`, `127/8`, `169.254/16` samt Cloud-Metadaten,
///   `172.16/12`, `192.168/16`, `::`, `::1`, `fc00::/7`, `fe80::/10`, Multicast), auch in eingebetteter Form
///   (`::ffff:a.b.c.d`, `64:ff9b::/96`, 6to4 `2002::/16`); ein System-Proxy nur, wenn er dieselbe Prüfung macht;
///   "privates Netz erlaubt" gibt es nur als Betreiber-Schalter des Programms (Home-Server), nie im Trait;
/// - Fehlercodes (`FetchState::Failed.reason`, `Loss.reason`, `Problem`) nennen nie die Adresse.
///
/// Größen- und Zeitgrenzen des Abrufs setzt das Programm. Die Adresse kann ein Geheimnis
/// tragen ("geheime Adresse"): sichtbar nur für Personen mit `calendar.update` an diesem Kalender (sonst `None`), nie im
/// Verlauf als Klartext (dort nur "geändert"), nie in `Origin.source` oder einem Export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subscription { pub url: Option<Reference>, pub refresh: Refresh, pub state: FetchState }

/// Wann ein Abo abgerufen wird. Abrufe gehen nur an die eine Adresse des Abos, nie anderswohin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refresh {
    /// Nur auf Klick (`refresh_calendar`). Vorgabe.
    Manual,
    /// Zusätzlich regelmäßig alle `seconds` Sekunden (der Anbieter darf eine Mindestzeit setzen, dann `Invalid`).
    Every { seconds: u32 },
}

/// Zustand des letzten Abrufs bzw. Abgleichs. Ein Fehler löscht nichts: die Termine des letzten guten Abrufs bleiben.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FetchState {
    /// Noch nie abgerufen.
    Never,
    /// Letzter Abruf gelungen.
    Fresh { at: Utc },
    /// Letzter Abruf gescheitert (`reason` = Code, kein Satz, nie die Adresse); `attempts` gescheiterte Abrufe in Folge.
    Failed { at: Utc, reason: String, last_success: Option<Utc>, attempts: u32 },
}

/// Änderung eines Kalenders: `Some` = setzen, `None` = bleibt. `url`/`refresh` nur bei einem Abo, sonst `Invalid`.
/// `owner` = Übergabe an eine andere Person oder Gruppe (Recht aus fs-access; an eine KI `Forbidden{calendar.update}`,
/// an ein `Profile` `Invalid`).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CalendarPatch {
    pub name: Option<String>,
    pub color: Option<Option<String>>,
    pub default_zone: Option<Option<Zone>>,
    pub owner: Option<PersonRef>,
    pub url: Option<Reference>,
    pub refresh: Option<Refresh>,
}

/// Bis wann Gelöschtes zurückgeholt werden kann (Papierkorb, ADR 0073 Nachtrag 2026-11-16). Die Frist ist eine
/// Einstellung des Programms (Pflicht; Standard 30 Tage, mindestens 1 Tag, "nie automatisch löschen" erlaubt), nicht
/// des Traits. Die Zusage gilt: nichts geht früher weg. Nach Ablauf geht das Objekt samt Verlauf endgültig (Kaskade).
/// Vorrang: eine Aufbewahrung (`keep_until`, fs-objects `object_hold`) geht vor Frist UND vor "nie automatisch":
/// solange sie gilt, ist es `Held`; danach `Until` bzw. `Manual`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Restorable {
    /// Keine Aufbewahrung; die Frist läuft bis dahin ("noch N Tage").
    Until(Utc),
    /// Aufbewahrt ("aufbewahrt bis …"): `Some(t)` = bis zum SPÄTEREN von Frist und Aufbewahrungsende (so liefert es
    /// fs-objects `restorable_until`); `None` = Aufbewahrung ohne Ende. `purge` ist dann `Forbidden`.
    Held(Option<Utc>),
    /// "Nie automatisch löschen" ist eingestellt, keine Aufbewahrung: bleibt, bis jemand `purge` aufruft.
    Manual,
}

/// Ergebnis von `delete_calendar`: der Kalender und `appointments` Termine darin liegen im Papierkorb, alle mit
/// derselben Frist; ihr Verlauf geht mit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarDeleted { pub id: ObjectId, pub restorable: Restorable, pub appointments: u32 }

/// Ergebnis eines Abrufs (`refresh_calendar`): neuer Zustand des Kalenders (auch bei Fehler: `FetchState::Failed`),
/// was neu kam (`report`, Verluste je Feld wie beim Import), was sich geändert hat (`updated`) und welche Termine die
/// Quelle nicht mehr hat (`removed`; sie gehen in den Papierkorb, nicht endgültig).
#[derive(Clone, Debug)]
pub struct Refreshed { pub calendar: CalendarInfo, pub report: ImportReport, pub updated: Vec<ObjectId>, pub removed: Vec<ObjectId> }
