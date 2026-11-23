//! Thema `groups`: EIN Vertrag (nach Nutzer-Entscheid 23.11.2026 keine Versionen nebeneinander; es gibt noch keine
//! Nutzer, nichts ist zu schonen). Eigentümer: fs-groups (ADR 0050, 0055: Quelle der Wahrheit für Gruppen). Traits
//! `Groups`, `GroupGovernance` (Eigentümer, Stufen, Eingriff, Zusätze), `Mirroring`, `GroupMembers`; ein Anbieter
//! trägt alle (gleiche Verbindung). Die Vertragsnummer für den Handshake ist 1.
//!
//! Der Vertrag gilt als Ganzes und ist NICHT gesperrt (Sperre aufgehoben, Nutzerwort 23.11.2026). Dateien (jede
//! <= 300 Zeilen, ADR 0043): `mod.rs` (dieser Kopf), `shared.rs` (`pub use` der Basistypen aus `fs-types`), `group.rs`,
//! `nesting.rs` (Kante, Modus, Zustimmung, Regel der wirksamen Mitglieder), `members.rs`, `rights.rs`, `errors.rs`,
//! `api.rs` (Trait `Groups`), `api_governance.rs` (Trait `GroupGovernance`), `level.rs` (Stufen, Rollen-IRIs, Bereich),
//! `admin.rs` (Bereich, Eingriff), `addon.rs` (Zusätze), `mirror.rs` (Trait `Mirroring`, verbrauchter Trait
//! `GroupDirectory`), `provided.rs` (Trait `GroupMembers` für Kalender und Tasks), `consumed.rs` (die Ports
//! `PersonDirectory`, `PersonFacts`, `RoleBook`, die der Gruppen-Vertrag selbst führt).
//!
//! Pattern: Repository + Port (Trait), Composition (Zusätze hängen an der Gruppe, kein Typ darüber),
//! Specification (Stufen und Schutz der letzten Eigentümerin sind in jedem Anbieter gleich, Konformitätstests).
//! Jede Methode ist Pflicht, kein Standard `not_possible`, keine Fähigkeitsabfrage zur Laufzeit, kein `Unsupported`:
//! was die fragende Person nicht darf, ist `Error::Forbidden` (außer: KI und Admin-/sensible Gruppe, dann
//! `NotFound`, `nesting.rs`), und das Frontend fragt vorher `my_rights` und bietet die Handlung dann nicht an.
//!
//! Regeln, die der Vertrag festlegt:
//! - Wer handelt, ist Eigenschaft des Anbieters (die fragende Person), nie ein Parameter. `may_do` fragt nach
//!   einer beliebigen Person, schreibt aber nichts. Die Uhr ist ebenfalls Eigenschaft des Anbieters.
//! - Jedes Schreiben auf ein bestehendes Objekt ist Vergleich-und-Setzen gegen `expected: Revision`
//!   (`Error::Changed`). Ausgenommen: Anlegen und `restore`; `restore` meldet `Changed`, wenn die Gruppe seit dem
//!   Löschen geändert wurde.
//! - Löschen einer Gruppe = Papierkorb mit Frist (`restorable_until`); `restore` holt die Gruppe samt
//!   Mitgliedschaften, Einladungen, Rechten und Kanten zurück. Eine Gruppe mit Eigentümer löscht nur ein Eigentümer
//!   (oder ein Eingriff).
//! - Menschen sind `PersonRef` (ADR 0085), nie ein freier Name, nie eine E-Mail-Adresse. Mitglied ist nur ein echtes
//!   Konto, nie ein `Profile`, nie ein KI-Konto (KI nur über den Bot-Platz, ADR 0091, 0095: keine KI-Gruppen).
//! - Eine Gruppe hat die Identität `urn:uuid:<UUIDv7>` (ADR 0027, 0085). Eigentümer eines Objekts ist eine Person
//!   ODER eine Gruppe (`PersonRef` mit `art = Group`); `resolve_owner` löst es zu den Personen auf.
//! - Eigentümer (ADR 0095): mehrere, Rolle `owner` (`role_owner`); die Rolle `owner` und die Rolle `manager` vergibt
//!   nur dieser Vertrag über seine eigenen Wege (`create`, `appoint_owner`, `transfer_ownership`; `manager`
//!   nur mit `Manage`): `add_member`, `set_role` und `invite` lehnen `owner` immer ab (`Forbidden`), `manager` ohne
//!   `Manage` ebenso. Die letzte Eigentümerin bleibt (`LastOwner`, auf dem Weg über `remove_member` `LastManager`).
//! - Verschachtelung hat EINEN Ort: die Kante (`Nesting`, Modus je Kante `BottomUp`/`TopDown`/`Single`;
//!   `default_mode` der oberen Gruppe ist nur die Voreinstellung für `nest_default`). Eine Gruppe ist nie Mitglied
//!   (`Error::WrongArt`); sie kann aber Inhaberin eines Rechts sein. Kreise sind über alle Modi verboten
//!   (`Error::Cycle` mit Pfad, ADR 0047, 0056). Jede Kante braucht die Zustimmung beider Seiten (ADR 0056); Lösen
//!   und Abschwächen geht einseitig. `BottomUp` (wie Kanidm): Mitglieder zählen nach oben, die Rechte der oberen
//!   Gruppe gelten für alle darunter. `TopDown`: Mitgliedschaft fließt nach unten, Rechte nach oben. `Single`: nur
//!   Baum, nichts fließt (in Kanidm flach, nicht gespiegelt). Ebenen (lokal, regional, ...) sind Kanten, kein Typ.
//! - Drei Stufen (ADR 0095): (c) jede Gruppe verwaltet sich selbst; (b)/(a) Gruppen mit einem `Scope` greifen nur
//!   über einen `Intervention` mit Grund ein (sichtbar im Protokoll, endet nach einer Stunde).
//! - Zusätze (`Addon`) per Komposition: die Gruppe ist der Kern; "Organisation" (W3C ORG `org:FormalOrganization`)
//!   ist ein angehängter Zusatz, keine Art der Gruppe. Nichts im Vertrag hängt von einem Zusatz ab.
//! - Eine KI hat an Gruppen mit Admin-Recht und an sensiblen Gruppen gar keinen Zugriff und sieht sie nicht
//!   (jede Methode antwortet wie für eine nicht vorhandene Id; Details `nesting.rs`). Lesen, was die fragende
//!   Person nicht sehen darf, ist `Withheld`, nie eine leere Liste.
//! - Reihenfolge von `Vec`-Feldern ist nicht Vertrag, außer wo das Doc sie nennt. Ein fallengelassenes Future bricht
//!   kein mehrschrittiges Schreiben ab (Kaskaden laufen zu Ende).
//!
//! Async (`async-trait`, ADR 0070). Speicherneutral: der Anbieter `LocalGroups` hält typisierte Tabellen `groups_*`
//! in fs-objects v5 (ADR 0086). Nicht im Vertrag: Folgen zwischen unabhängigen Gruppen, Import/Export (Grenz-Adapter),
//! Gruppenschlüssel (MLS, ADR 0067).
//!
//! Wer implementiert: `LocalGroups` (später), `RemoteGroups` (Client über `groups.capnp`),
//! `FakeGroups`/`FailingGroups`/`SlowGroups`/`EmptyGroups` (dieses Crate).

// Gezielte Ausnahmen von clippy::pedantic (Regel der Strategie: nur mit Begründung):
// - module_name_repetitions: die Typnamen sind Vertrag (`MirrorStep` in `mirror.rs`), alles wird flach über
//   `pub use` angeboten; der Modulname ist nur Ablage.
// - missing_errors_doc: jede Methode nennt ihre Fehler im Doc-Satz, die Ursachen stehen typisiert in
//   `errors.rs`; eine `# Errors`-Rubrik je Methode wiederholte nur.
#![allow(clippy::module_name_repetitions, clippy::missing_errors_doc)]

mod addon;
mod admin;
mod api;
mod api_governance;
mod consumed;
mod errors;
mod group;
mod level;
mod members;
mod mirror;
mod nesting;
mod provided;
mod rights;
pub mod shared;

pub use addon::*;
pub use admin::*;
pub use api::*;
pub use api_governance::*;
pub use consumed::*;
pub use errors::*;
pub use group::*;
pub use level::*;
pub use members::*;
pub use mirror::*;
pub use nesting::*;
pub use provided::*;
pub use rights::*;
pub use shared::*;
