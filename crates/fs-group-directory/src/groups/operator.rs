//! Betreiber: die Gruppe mit dem Bereich `Server` (Nutzerentscheid 16, 23.11.2026; ADR 0095 Nachtrag).
//!
//! Wer Server-/root-Zugriff hat, kann alles lesen; der Vertrag sagt das offen (`GroupGovernance::operators` ist für
//! jede Person lesbar). Die Rolle `operator` setzt NUR der lokale Befehl auf dem Server (Workflow
//! server-einrichtung, ein Mensch führt ihn aus): dafür gibt es hier den Eingang [`LocalOperator`], der NICHT zu
//! `GroupGovernance` gehört, keine Methode in `groups.capnp` hat und von `RemoteGroups` nie angeboten wird. Es gibt
//! keinen Vertragsweg zum Betreiber: kein `add_member`, `set_role`, `invite`, `appoint_owner`, `grant_scope` oder
//! Eingriff (auch nicht der Stufen (a)/(b)) erreicht die Gruppe oder die Rolle. Einzelplatz (ADR 0090): die eine
//! Person ist Betreiberin (der lokale Befehl läuft bei der Einrichtung für sie). Bot/KI und betreute Konten sind nie
//! Betreiber, auch nicht über den lokalen Befehl ([`check_operator_candidate`], fail-closed).
//!
//! Pattern: Specification (die Prüfung ist eine reine Funktion, jeder Anbieter ruft dieselbe) + getrennter Port
//! (Interface Segregation: der lokale Eingang ist ein eigener Trait).

use super::shared::{ObjectId, PersonArt, PersonRef};
use super::{Error, Result};
use async_trait::async_trait;

/// Die Betreiber, sichtbar für jede Person (auch für eine KI: es ist eine offene Liste, kein Geheimnis).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OperatorList {
    /// Die Gruppe „Betreiber“; `None`, solange der lokale Befehl noch nie gelaufen ist.
    pub group: Option<ObjectId>,
    /// Die wirksamen Betreiber (Mitgliedschaft mit der Rolle `operator`, ein bekannter, nicht betreuter Mensch);
    /// älteste Mitgliedschaft zuerst.
    pub persons: Vec<PersonRef>,
}

/// Die Prüfung, wer Betreiber werden darf (der lokale Befehl und jeder Anbieter ruft sie; nichts anderes setzt
/// Betreiber). `agent`: Markierung KI (`PersonFacts::is_agent`), `supervised`: betreut (`is_supervised`).
/// Nur ein Konto ist Betreiber (nie `Profile`, `Group`, `Organization`), ein bekannter Mensch, nicht betreut.
/// Unbekannt heißt nein.
pub fn check_operator_candidate(
    art: PersonArt,
    agent: Option<bool>,
    supervised: Option<bool>,
) -> Result<()> {
    if matches!(
        art,
        PersonArt::Group | PersonArt::Organization | PersonArt::Profile
    ) {
        return Err(Error::WrongArt {
            field: super::Field::Person,
            got: art,
        });
    }
    match agent {
        Some(false) => {}
        Some(true) => return Err(Error::AgentNotAdmin),
        None => return Err(Error::AiStatusUnknown),
    }
    if supervised == Some(false) {
        Ok(())
    } else {
        Err(Error::SupervisedCannotOperate)
    }
}

/// Namen, die nur die Gruppe der Betreiber trägt (nach `fold_name`): eine Gruppe, die sich so nennt, täuschte Menschen,
/// denn die Oberfläche nennt die Betreiber mit Namen.
const RESERVED_NAMES: [&str; 7] = [
    "betreiber",
    "betreiberin",
    "betreiberinnen",
    "serverbetreiber",
    "operator",
    "operators",
    "serveroperator",
];

/// Der Name ohne Schreibvarianten: Voll- zu Halbbreite (U+FF01..FF5E), nur Buchstaben und Ziffern (Leerzeichen,
/// Satzzeichen, Bindestriche, unsichtbare und kombinierende Zeichen fallen weg), Kleinschreibung nach Unicode.
/// Nicht gedeckt: Zeichen, die nur ähnlich aussehen (kyrillisches `е` statt `e`) und Übersetzungen in andere Sprachen.
fn fold_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '\u{FF01}'..='\u{FF5E}' => char::from_u32(u32::from(c) - 0xFEE0).unwrap_or(c),
            _ => c,
        })
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Ob ein Gruppenname (oder eine Gruppenart) für die Gruppe der Betreiber reserviert ist. Der Anbieter ruft sie bei
/// `create` und `update` für `name` und `kind`; die Betreiber-Gruppe selbst legt nur der lokale Befehl an. Wer die
/// Betreiber zeigt, nimmt sie aus `operators()`, nie aus einem Namen.
#[must_use]
pub fn is_reserved_group_name(name: &str) -> bool {
    let folded = fold_name(name);
    RESERVED_NAMES.contains(&folded.as_str())
}

/// NUR LOKAL: der Eingang des lokalen Befehls auf dem Server (gehört dem Server-Programm; das Programm ruft ihn nach
/// dem Beleg „läuft als root auf diesem Server“). KEIN Teil von `GroupGovernance`, nicht über Netz, MCP oder
/// `groups.capnp` erreichbar; `RemoteGroups` implementiert ihn nie. Wer ihn aufrufen kann, hat ohnehin root und könnte
/// alles lesen: das Recht gilt genau so weit wie die technische Macht.
#[async_trait]
pub trait LocalOperator: Send + Sync {
    /// Eine Person zur Betreiberin machen (legt die Gruppe „Betreiber“ mit dem Bereich `Server` beim ersten Mal an).
    /// Lehnt ab: [`check_operator_candidate`] (KI, unbekannt, betreut, kein Konto) und eine unbekannte Person
    /// (`NotFound`); eine schon eingetragene Person ist `AlreadyThere`.
    async fn operator_set_local(&self, person: &PersonRef) -> Result<()>;
    /// Eine Betreiberin austragen (ebenfalls nur lokal; auch die letzte). Keine Betreiberin: `NotFound`.
    async fn operator_unset_local(&self, person: &PersonRef) -> Result<()>;
}
