//! Thema `groups`, Vertrag v2 (ADR 0095, Entscheid groups G2 und Nutzer-Entscheid 23.11.2026): neben dem gesperrten
//! `groups_v1/`, nur angehängt. `GroupsV2` ist ein eigener Trait NEBEN `Groups` (derselbe Anbieter implementiert beide): alles aus v1 (Mitglieder, Kanten mit
//! Modus je Kante, `default_mode` je Gruppe, Rechte, Einladungen, Papierkorb) gilt weiter; v2 fügt an, was v1 nicht
//! kann.
//!
//! Pattern: Repository + Port (Trait), Composition (Zusätze hängen an der Gruppe, kein Typ darüber), Specification
//! (die Stufen und der Schutz der letzten Eigentümerin sind in jedem Anbieter gleich, Konformitätstests).
//!
//! Was v2 festlegt:
//! - **Eigentümer** (ADR 0095): mehrere; Eigentümer = direktes, menschliches Mitglied mit der Rolle [`role_owner`];
//!   `Own` gibt es nur über diese Rolle. Wer eine Gruppe mit `create_owned` anlegt, ist erste Eigentümerin.
//!   Ernennen, Herabstufen, Übergeben: nur Eigentümer (`NeedsOwner`). Die letzte Eigentümerin bleibt (`LastOwner`).
//! - **Drei Stufen**: (c) jede Gruppe verwaltet sich selbst (Eigentümer, Verwalter, auch die Zusätze); (b)/(a) Gruppen
//!   mit einem [`Scope`] (`AllGroups`, `Project`) greifen nur über einen [`Intervention`] mit Grund ein, sichtbar im
//!   Protokoll der Gruppe, endet nach einer Stunde.
//! - **Vererbung**: der Modus gilt je Kante (`request_nesting(child, parent, mode)` in v1); `default_mode` der oberen
//!   Gruppe ist nur die Voreinstellung für `nest_default`.
//! - **Mitglied = `PersonRef`** (nie ein Typ aus fs-people-core); **Eigentümer eines Objekts = Person ODER Gruppe**
//!   (`PersonRef` mit `art = Group`, technical.md 3a); `resolve_owner` löst das zu den Personen auf.
//! - **Zusätze** ([`Addon`]) per Komposition: die Gruppe ist der Kern, "Organisation / juristische Person" ist ein
//!   angehängter Zusatz (W3C ORG `org:FormalOrganization`), mehrere je Gruppe, offen erweiterbar über eine IRI.
//!   Nichts im Vertrag hängt von einem Zusatz ab: Mitglieder, Rechte und Vererbung sind mit und ohne gleich.
//! - **Ports bleiben im Gruppen-Vertrag**: `PersonDirectory`, `PersonFacts`, `RoleBook` (aus `groups_v1::consumed`,
//!   hier wieder angeboten; `person_directory_v1` liefert weder "ist KI" noch das Rollenbuch).
//! - **Keine KI-Gruppen**: KI nur über den Bot-Platz, nie als Gruppe oder Mitglied-Konto (ADR 0091).
//! - **Wer handelt** ist die Verbindung des Anbieters, nie ein Argument (wie v1). Lesen, was die fragende Person nicht
//!   sehen darf, ist `Withheld`, nie eine leere Liste.

mod addon;
mod admin;
mod api;
mod errors;
mod level;

pub use addon::*;
pub use admin::*;
pub use api::*;
pub use errors::*;
pub use level::*;

/// Die Ports, die der Gruppen-Vertrag braucht und selbst führt (nicht in `person_directory_v1`).
pub use crate::groups_v1::{PersonDirectory, PersonFacts, RoleBook, RoleInfo};
