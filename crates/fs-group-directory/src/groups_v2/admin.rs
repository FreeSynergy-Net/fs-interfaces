//! Verwaltungsstufen (a) und (b) als Gruppen mit Geltungsbereich, und der Eingriff (ADR 0095 Pkt. 5, 7).
//!
//! Pattern: Value Object (Bereichsvergabe, Eingriff) + Proxy (Eingriff: jede Handlung läuft durch dieselbe Prüfung
//! und steht im Protokoll der betroffenen Gruppe).

use super::Scope;
use crate::groups_v1::HistoryKind;
use crate::groups_v1::shared::{ObjectId, PersonRef, Reference, Revision, Utc};

/// Ein Eingriff schließt nach einer Stunde von selbst (ADR 0095 Entschieden 11).
pub const INTERVENTION_MS: i64 = 60 * 60 * 1000;

/// Ein Geltungsbereich, einer Admin-Gruppe vergeben. Die Bereiche hält immer eine Gruppe, nie eine Person.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeGrant {
    pub id: ObjectId,
    pub revision: Revision,
    /// Die Admin-Gruppe, die den Bereich hält.
    pub admin_group: ObjectId,
    /// `Project` oder `AllGroups` (`Group(_)` wird nie vergeben).
    pub scope: Scope,
    pub granted_by: PersonRef,
    pub granted: Utc,
}

/// Was mit einem Zusatz geschah (v2-eigene Handlung: `HistoryKind` aus `groups_v1` ist gesperrt).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddonVerb {
    Attached,
    Updated,
    Detached,
}

/// Die Art einer Handlung innerhalb eines Eingriffs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActKind {
    /// Eine Handlung, die auch das Gruppen-Protokoll kennt.
    History(HistoryKind),
    /// Ein Zusatz (`iri` = seine Art) wurde angehängt, geändert oder gelöst (ADR 0095 Pkt. 7, kein stiller Weg).
    Addon { verb: AddonVerb, iri: Reference },
}

/// Eine Handlung innerhalb eines Eingriffs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterventionAct {
    pub at: Utc,
    pub kind: ActKind,
}

/// Ein Eingriff von oben in eine Gruppe: Grund Pflicht, sichtbar für jede Person, die die Gruppe lesen darf.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Intervention {
    pub id: ObjectId,
    /// Die Gruppe, in die eingegriffen wird.
    pub group: ObjectId,
    /// Wer eingreift (AS2 `actor`).
    pub actor: PersonRef,
    /// Die Admin-Gruppe, aus der der Bereich stammt.
    pub admin_group: ObjectId,
    pub scope: Scope,
    pub reason: String,
    pub opened: Utc,
    pub ends: Utc,
    /// `Some`, wenn er vorzeitig geschlossen wurde.
    pub closed: Option<Utc>,
    pub acts: Vec<InterventionAct>,
}
