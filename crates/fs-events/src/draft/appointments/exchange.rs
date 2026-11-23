//! Grenze (ADR 0085 Abschnitt 1, Strang D E15): Formate liefern Anbieter außerhalb des Fach-Traits.
//!
//! Pattern: Adapter (Formate an der Grenze, hinter einem Trait); jede Methode ist Pflicht. Personen kommen über
//! `PersonDirectory` aus `person_directory_v1` (fs-people), den der Anbieter im Konstruktor bekommt (verbraucht, wie
//! `tasks_v1`); dieser Ordner hat keinen eigenen Personen-Port und importiert jenen Trait nicht (Verweis nur im Doc).

use super::{FileRef, MediaType, ObjectId, Problem, Reference, Result, Zone};
use async_trait::async_trait;

/// Fähigkeit eines Formats je Richtung.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Level { Full, Partial { missing: Vec<String> }, No }
/// Was ein Format-Anbieter kann.
#[derive(Clone, Debug)]
pub struct FormatSupport { pub media_type: MediaType, pub import: Level, pub export: Level }
/// Was beim Umwandeln nicht abbildbar war (nie still verloren). `reason` ist ein Code, nie eine Adresse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loss { pub uid: Option<String>, pub field: String, pub reason: String }
/// Ergebnis eines Imports.
#[derive(Clone, Debug, Default)]
pub struct ImportReport {
    pub created: Vec<ObjectId>,
    pub skipped: Vec<String>,
    pub failed: Vec<Problem>,
    pub lost: Vec<Loss>,
    /// Zeiten, deren Zone eingesetzt wurde (`zone_assumed`), je `uid`.
    pub zone_assumed: Vec<String>,
    /// Sammelvorschlag Personen (ADR 0058/0085): die Id aus `person_directory_v1` `open_batch(origin)`, Herkunft =
    /// die Datei (`urn:sha256:…`) bzw. Konto-IRI; `None`, wenn keine unbekannte Adresse vorkam.
    pub people_proposal: Option<ObjectId>,
    /// Aufgaben aus der Datei, weitergegeben an fs-tasks (ADR 0050).
    pub forwarded_tasks: Vec<String>,
}
/// Ziel eines Exports: Medientyp und, bei Konten, die Quell-IRI. Ziel = Quelle entscheidet über die
/// Rundlauf-Regel (ADR 0085 Abschnitt 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportTarget { pub media_type: MediaType, pub destination: Option<Reference> }
/// Ergebnis eines Exports.
#[derive(Clone, Debug)]
pub struct ExportReport { pub written: FileRef, pub lost: Vec<Loss> }
/// Grundlage der Normalisierung beim Import: Zone für schwebende Zeiten (Einstellung der Person, sonst
/// Gerät, sonst UTC mit Meldung; ADR 0085 "Gerätezone fehlt").
#[derive(Clone, Debug)]
pub struct ImportContext { pub calendar: ObjectId, pub assume_zone: Zone, pub assume_zone_source: ZoneSource }
/// Woher die eingesetzte Zone stammt (wird gemeldet).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneSource { PersonSetting, Device, LastKnownDevice, UtcFallback }

/// Grenz-Adapter eines Formats (`text/calendar`, `application/jscalendar+json`, später iTIP/iMIP).
/// Liest Fremdformat, normalisiert (Zeiten; Personen über `person_directory_v1`: bekannte Adresse =
/// `find_by_address`, erster Treffer; unbekannte = `propose` mit `ProposeEntry.origin` = Herkunft der Datei in EINEM
/// Stapel aus `open_batch`, nie freier Name) und schreibt über [`super::Appointments`]; Export ist ein eigener
/// Befehl, nie Speichern (ADR 0046). Ein Anbieter ohne eine Richtung sagt es in `formats` (`Level::No`) und antwortet
/// mit [`super::not_possible`].
#[async_trait]
pub trait Exchange: Send + Sync {
    /// Was der Anbieter je Medientyp kann.
    fn formats(&self) -> Vec<FormatSupport>;
    /// `exchange.import`; eine unbekannte oder eigene Zone wird ersetzt (`zone_assumed`, `Loss`), nie geraten.
    /// Fehler des Verzeichnisses (auch `Forbidden` für eine KI: `person.find`, `person.propose`) bricht den ganzen Import ab und wird durchgereicht; nie still als unbekannte Adresse behandeln, nie ohne Personen importieren.
    async fn import(&self, source: FileRef, as_type: &MediaType, ctx: &ImportContext) -> Result<ImportReport>;
    /// `exchange.export`: Ziel = Quelle und unverändert = Original Byte für Byte; Ziel = Quelle, nur
    /// `zone_assumed`-Zeit unangetastet = wieder schwebend; sonst mit Zone (Rundlauf-Regel ADR 0085).
    async fn export(&self, ids: &[ObjectId], to: &ExportTarget) -> Result<ExportReport>;
}
