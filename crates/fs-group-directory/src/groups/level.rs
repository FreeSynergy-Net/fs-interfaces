//! Stufen, Rollen-IRIs und Geltungsbereich (ADR 0095 Pkt. 1, 3, 5).
//!
//! Pattern: Value Object (Stufe, Bereich); Rollen sind Verweise auf das Rollenbuch, nie ein geschlossener Typ.

use crate::groups::RoleRef;
use crate::groups::shared::{BasicValue as _, ObjectId, Reference};

/// Basis der Rollen-IRIs (`…/roles/member`, `…/manager`, `…/owner`), flach, Art `org:Role`.
pub const ROLE_BASE: &str = "https://freesynergy.net/ns/roles/";

fn role(name: &str) -> RoleRef {
    RoleRef(Reference::parse(&format!("{ROLE_BASE}{name}")).expect("a role IRI is a reference"))
}

/// Die Rolle Eigentümer: gibt `Own` (schließt `Manage` ein), nur über diese Rolle.
#[must_use]
pub fn role_owner() -> RoleRef {
    role("owner")
}

/// Die Rolle Betreiber (Bereich `Server`, ADR 0095 Nachtrag): nur der lokale Befehl auf dem Server vergibt sie
/// (`LocalOperator`, `operator.rs`), nie ein Weg dieses Vertrags.
#[must_use]
pub fn role_operator() -> RoleRef {
    role("operator")
}

/// Die Rolle Verwalter: gibt `Manage`.
#[must_use]
pub fn role_manager() -> RoleRef {
    role("manager")
}

/// Die Stufe der fragenden Person in einer Gruppe (geordnet: `Owner` > `Manager` > `Member` > `None`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Level {
    /// Kein direktes Mitglied und kein Recht.
    None,
    /// Direktes Mitglied ohne Verwaltung.
    Member,
    /// Verwaltet (Rolle `manager` oder Recht `Manage`).
    Manager,
    /// Eigentümerin (Rolle `owner`).
    Owner,
}

/// Name der Gruppe, die den Bereich `Server` hält (Mitglieder = die Betreiber, immer sichtbar).
pub const OPERATOR_GROUP_NAME: &str = "Betreiber";

/// Geltungsbereich eines Rechts (ADR 0095 Pkt. 5; `Server` nach Nutzerentscheid 16, 23.11.2026).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    /// (a) Projekt-Verwaltung: alles.
    Project,
    /// (b) Verwaltung aller Gruppen.
    AllGroups,
    /// (c) Selbstverwaltung: nur diese Gruppe (nie vergeben, nur genannt).
    Group(ObjectId),
    /// Betreiber: wer Server-/root-Zugriff hat. Kein Eingriff-Bereich (der Betreiber liest ohne Eingriff, ändert aber nur
    /// über einen Eingriff der Stufen (a)/(b)). NIE über den Vertrag vergebbar: `grant_scope` sagt `ScopeDenied`, kein
    /// anderer Bereich deckt ihn ab; gesetzt wird er nur vom lokalen Befehl (`LocalOperator`).
    Server,
}

impl Scope {
    /// Ob ein gehaltener Bereich den gewünschten abdeckt (`Project` alles außer `Server`, `AllGroups` jede Gruppe;
    /// `Server` deckt nur sich selbst und wird von keinem anderen Bereich gedeckt).
    #[must_use]
    pub fn covers(self, wanted: Scope) -> bool {
        matches!(
            (self, wanted),
            (
                Scope::Project,
                Scope::Project | Scope::AllGroups | Scope::Group(_)
            ) | (Scope::AllGroups, Scope::AllGroups | Scope::Group(_))
        ) || self == wanted
    }
}
