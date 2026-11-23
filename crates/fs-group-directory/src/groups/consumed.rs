//! VERBRAUCHT, nicht unser Thema: was fs-groups von fs-people braucht (ADR 0052, 0056, 0085). Eigentümer:
//! fs-people. Stand-in der Form, die fs-groups braucht; die Brücke `fs-groups-people` (Adapter) setzt sie auf
//! die echten Schnittstellen von fs-people um. Weicht fs-people ab, ändert sich nur die Brücke, nicht dieser
//! Ordner, solange die Form hier erfüllbar bleibt.
//!
//! Pattern: Adapter (Personenverzeichnis, Rollenbuch und KI-Markierung an der Grenze, hinter Traits).

use super::shared::PersonRef;
use super::{Result, RoleRef};
use async_trait::async_trait;

/// Personen-Verweis auflösen. GLEICHE Form wie `appointments_v1::PersonDirectory`, damit fs-people
/// EINEN Anbieter für Kalender und Gruppen baut.
#[async_trait]
pub trait PersonDirectory: Send + Sync {
    /// Person zu einer Adresse (`mailto:…`), wenn bekannt.
    async fn find_by_address(&self, uri: &str) -> Result<Option<PersonRef>>;
    /// Unbekannte Adresse: Eintrag in der lokalen Ablage im Zustand "vorgeschlagen" (nie freier Name).
    async fn propose(&self, display_hint: &str, uri: &str) -> Result<PersonRef>;
    /// Anzeigename und Adresse für die Oberfläche (aktuell, nicht eingefroren).
    async fn display(&self, person: &PersonRef) -> Result<Option<(String, Option<String>)>>;
}

/// Ob eine Person eine KI ist (ADR 0054, 0076: Markierung in fs-people). `None` = unbekannt; dann zählt sie für
/// Admin-Regeln wie eine KI (im Zweifel nein). `Some(false)` nur, wenn bekannt ist, dass es ein Mensch ist.
/// Die Markierung setzt in fs-people nur ein Mensch, nie die markierte Person selbst, und jeder Wechsel löst
/// eine Neubewertung aus (Mitgliedschaften der KI zählen dann nicht mehr, `InForce::KindMismatch`). Grenze
/// (ADR 0054): eine Markierung schützt nur vor ehrlichen KIs; wer angreift, markiert sich nicht.
#[async_trait]
pub trait PersonFacts: Send + Sync {
    async fn is_agent(&self, person: &PersonRef) -> Result<Option<bool>>;
}

/// Eine Rolle aus dem Rollenbuch (SKOS / `org:Role`, ADR 0052).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleInfo {
    pub role: RoleRef,
    /// Anzeigename in der Sprache der Person (fs-people liefert die Übersetzung, ADR 0057).
    pub label: String,
    /// Admin-Rolle: nie für KI oder unbekannte Personen (ADR 0056).
    pub admin: bool,
}

/// Rollen, die in Gruppen vergeben werden können (Definition in fs-people, ADR 0052).
#[async_trait]
pub trait RoleBook: Send + Sync {
    /// Alle Rollen für die Auswahl, in der Reihenfolge des Rollenbuchs.
    async fn roles(&self) -> Result<Vec<RoleInfo>>;
    /// Eine Rolle; `None`, wenn das Rollenbuch sie nicht (mehr) kennt (Anzeige: der Verweis selbst).
    async fn role(&self, role: &RoleRef) -> Result<Option<RoleInfo>>;
}
