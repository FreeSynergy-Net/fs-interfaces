//! Gruppe und Abfrage (Tabelle `groups_group`, PLAN.md §3). Die Verschachtelung steht in `nesting.rs`.
//!
//! Pattern: Value Object (Gruppe, Entwurf, Änderung, Abfrage).

use super::NestingMode;
use super::shared::{ObjectId, PersonRef, Reference, Revision, Utc};

/// Wer die Mitgliederliste sehen darf (Name und Art sieht jeder, der die Gruppe lesen darf). Der Anbieter
/// prüft es für die fragende Person (`Error::Forbidden { right: Right::Read }`; für eine KI an einer Admin- oder
/// sensiblen Gruppe `NotFound`, `nesting.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Alle, die den Anbieter erreichen.
    Everyone,
    /// Nur wirksame Mitglieder (auch geerbte) und Inhaber von `Right::Read` oder mehr.
    Members,
    /// Nur Inhaber von `Right::Manage`.
    Managers,
}

/// Eine Gruppe. Eltern und Kinder: `Groups::nestings`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub id: ObjectId,
    /// `urn:uuid:<id>`; Ziel jedes `PersonRef { art: Group }` auf diese Gruppe| Organization }` auf diese Gruppe.
    pub reference: Reference,
    pub revision: Revision,
    /// Anzeigename, nicht leer.
    pub name: String,
    /// Freie Bezeichnung der Person ("Team", "Abteilung", "Verein"); keine Logik hängt daran.
    pub kind: Option<String>,
    pub description: Option<String>,
    /// Modus des Baums unter dieser Gruppe: vorgewählt für neue Kinder. Es gilt immer der Modus der einzelnen
    /// Kante; "Modus des Baums ändern" ändert jede Kante darunter (`change_nesting_mode`). `BottomUp` (Teams in einem Bereich):
    /// Mitglieder zählen nach oben, die Rechte der oberen Gruppe gelten für alle darunter. `TopDown`:
    /// Mitgliedschaft fließt nach unten, Rechte fließen nach oben (Chefs in `member` von Vizechefs: Chefs
    /// dürfen, was die Vizechefs dürfen, und sind auch deren wirksame Mitglieder).
    pub default_mode: NestingMode,
    pub visibility: Visibility,
    /// Besondere Kategorie (DSGVO Art. 9, z. B. Gewerkschaft, Glaube, Gesundheit): Mitgliedschaften gelten als
    /// sensibel (ADR 0029), werden nicht exportiert, nie gespiegelt; kein Bogen des Mitgliedsgraphen zwischen
    /// sensibel und nicht sensibel, in beiden Richtungen (`Error::ModeConflict`). `Single` bleibt erlaubt. Eine KI
    /// sieht sensible Gruppen nicht (`NotFound`, nicht in Listen) und hat keinen Zugriff.
    pub sensitive: bool,
    /// Soll diese Gruppe in Verzeichnisse (Kanidm) gespiegelt werden (ADR 0056 Punkt 8: nur wer Rechte braucht)?
    /// Bei `sensitive` immer `false` (`Error::Invalid`, `SensitiveNotMirrored`).
    pub mirror: bool,
    pub created: Utc,
    pub created_by: PersonRef,
    pub updated: Utc,
}

/// Zeile einer Liste. Zahlen sind `None`, wo die fragende Person die Mitglieder nicht sehen darf.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupSummary {
    pub id: ObjectId,
    pub reference: Reference,
    pub revision: Revision,
    pub name: String,
    pub kind: Option<String>,
    pub default_mode: NestingMode,
    /// Zahl der SICHTBAREN Eltern und Kinder (für eine KI zählen verborgene Gruppen nicht, `nesting.rs`).
    pub parents: u32,
    pub children: u32,
    pub direct_members: Option<u32>,
    pub sensitive: bool,
    pub mirror: bool,
    /// `None` = nicht gelöscht; sonst Ende der Papierkorbfrist.
    pub restorable_until: Option<Utc>,
}

/// Wo in der Verschachtelung gesucht wird (alle Modi).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParentFilter {
    Any,
    /// Gruppen ohne SICHTBAREN Elternteil (Wurzeln des Baums; für eine KI zählen verborgene Eltern nicht).
    Roots,
    /// Direkte Kinder dieser Gruppe.
    /// Für eine verborgene Id (KI) antwortet der Anbieter wie für eine unbekannte Gruppe (`nesting.rs`).
    ChildrenOf(ObjectId),
}

/// Abfrage für `list`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupQuery {
    /// Teil des Namens oder der Bezeichnung, ohne Groß/klein; leer = alle.
    pub text: String,
    pub parent: ParentFilter,
    /// Nur Gruppen, in denen diese Person DIREKT Mitglied ist.
    pub member: Option<PersonRef>,
    /// Auch Gruppen im Papierkorb (erkennbar an `restorable_until`).
    pub include_deleted: bool,
}

/// Neue Gruppe. Leere Felder: `Error::Invalid`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupDraft {
    pub name: String,
    pub kind: Option<String>,
    pub description: Option<String>,
    pub default_mode: NestingMode,
    pub visibility: Visibility,
    pub sensitive: bool,
    pub mirror: bool,
}

/// Änderung; `None` = Feld bleibt. `Some(None)` löscht ein freiwilliges Feld. `sensitive` kann
/// `Error::ModeConflict` ergeben (Bogen zwischen sensibler und nicht sensibler Gruppe, beide Richtungen).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroupPatch {
    pub name: Option<String>,
    pub kind: Option<Option<String>>,
    pub description: Option<Option<String>>,
    pub default_mode: Option<NestingMode>,
    pub visibility: Option<Visibility>,
    pub sensitive: Option<bool>,
    pub mirror: Option<bool>,
}

/// Ergebnis von `delete`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deleted {
    pub id: ObjectId,
    pub restorable_until: Utc,
}
