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

/// Geltungsbereich eines Rechts (ADR 0095 Pkt. 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scope {
    /// (a) Projekt-Verwaltung: alles.
    Project,
    /// (b) Verwaltung aller Gruppen.
    AllGroups,
    /// (c) Selbstverwaltung: nur diese Gruppe (nie vergeben, nur genannt).
    Group(ObjectId),
}

impl Scope {
    /// Ob ein gehaltener Bereich den gewünschten abdeckt (`Project` alles, `AllGroups` jede Gruppe).
    #[must_use]
    pub fn covers(self, wanted: Scope) -> bool {
        matches!(
            (self, wanted),
            (Scope::Project, _) | (Scope::AllGroups, Scope::AllGroups | Scope::Group(_))
        ) || self == wanted
    }
}
