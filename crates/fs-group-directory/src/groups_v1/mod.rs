//! Thema `groups`, Ziel v1 (Sprung von "groups (draft)", `groups.capnp` fs-interfaces). Eigentümer: fs-groups
//! (ADR 0050, 0055: fs-groups ist die Quelle der Wahrheit für Gruppen). Trait `Groups` (+ `Mirroring`).
//!
//! KANDIDAT 2, NICHT GESPERRT (Pilot fs-groups, Stufe 2, 2026-11-16; Kandidat 2 = Verschachtelung mit Modus
//! je Kante nach der Nutzerentscheidung O1). Gesperrt wird der ORDNER `groups_v1/` als Ganzes (ADR 0043: kleine
//! Dateien, jede <= 300 Zeilen). Hash = `/srv/FreeSynergy/.scratch/tools/hash-ordner.sh groups_v1`. Dateien:
//! `mod.rs` (dieser Kopf), `shared.rs` (`pub use` der Basistypen aus `fs-types`), `group.rs`, `nesting.rs`
//! (Kante, Modus, Zustimmung, Regel der wirksamen Mitglieder), `members.rs`, `rights.rs`, `errors.rs`, `api.rs`
//! (Trait `Groups`), `mirror.rs` (Trait `Mirroring`, verbrauchter Trait `GroupDirectory`), `provided.rs` (Trait
//! `GroupMembers` für Kalender und Tasks), `consumed.rs` (verbrauchte Traits von fs-people). Ein anderer Name,
//! Konstruktor oder Zugriff in `fs-types` ist ein Sprung (ADR 0045).
//!
//! Pattern: Repository + Port (Trait). Jede Methode ist Pflicht, kein Standard `not_possible`, keine
//! Fähigkeitsabfrage zur Laufzeit, kein `Unsupported` (Muster `packages_v1`, Ziel "keine toten Knöpfe"): was die
//! fragende Person nicht darf, ist `Error::Forbidden` (außer: KI und Admin-/sensible Gruppe, dann `NotFound`,
//! `nesting.rs`), und das Frontend fragt vorher `my_rights` und bietet die Handlung dann nicht an.
//!
//! Regeln, die der Vertrag festlegt (Lehren aus den Reviews von `appointments_v1` und `packages_v1`):
//! - Wer handelt, ist Eigenschaft des Anbieters (die fragende Person), nie ein Parameter. `may_do` fragt nach
//!   einer beliebigen Person, schreibt aber nichts. Die Uhr ist ebenfalls Eigenschaft des Anbieters (Ablauf von
//!   Einladungen, Papierkorbfrist); kein `now`-Parameter.
//! - Jedes Schreiben auf ein bestehendes Objekt ist Vergleich-und-Setzen gegen `expected: Revision`
//!   (`Error::Changed`). Ausgenommen: Anlegen (`create`, `add_member`, `invite`, `grant`, `request_nesting`) und
//!   `restore`; `restore` meldet `Changed`, wenn die Gruppe seit dem Löschen geändert wurde (Konformitätstest).
//! - Löschen einer Gruppe = Papierkorb mit Frist (`restorable_until`); `restore` holt die Gruppe samt
//!   Mitgliedschaften, Einladungen, Rechten und Kanten zurück. Endgültiges Löschen nach der Frist
//!   (ADR 0073 "wirklich löschen"). Mitglied entfernen und Recht entziehen haben keinen Papierkorb: der Rückweg
//!   ist erneut hinzufügen; das Protokoll (`history`) hält beides fest.
//! - Menschen sind `PersonRef` (ADR 0085), nie ein freier Name, nie eine E-Mail-Adresse. Eine unbekannte Adresse
//!   wird über `PersonDirectory::propose` zu einem vorgeschlagenen Eintrag in fs-people.
//! - Eine Gruppe hat die Identität `urn:uuid:<UUIDv7>` (ADR 0027, 0085); die `https`-Form bildet der Export.
//! - Verschachtelung hat EINEN Ort: die Kante (`Nesting`, Modus `BottomUp`/`TopDown`/`Single`). Eine Gruppe
//!   ist nie Mitglied (`Error::WrongArt`); sie kann aber Inhaberin eines Rechts sein. Kreise sind über alle Modi
//!   verboten (`Error::Cycle` mit Pfad, ADR 0047, 0056). Jede Kante braucht die Zustimmung beider Seiten
//!   (ADR 0056 "gilt erst, wenn beide zugestimmt haben"); Lösen und Abschwächen geht einseitig.
//!   `BottomUp` (Beispiel Helfa, wie Kanidm): Mitglieder zählen nach oben, die Rechte der oberen Gruppe gelten
//!   für alle darunter. `TopDown`: Mitgliedschaft fließt nach unten, Rechte fließen nach oben: die obere Gruppe
//!   steht in `member` der unteren (Beispiel: 'Chefs' steht in `member` von 'Vizechefs'; Chefs dürfen, was die
//!   Vizechefs dürfen, und sind auch wirksame Mitglieder der Vizechefs: sie sehen deren Mitgliederliste und
//!   werden mit eingeladen). `Single`: nur Baum, nichts fließt (Anzeigekante, in Kanidm flach, nicht
//!   gespiegelt). KI-Regel: KI-Konten nur in KI-Gruppen, KI-Gruppen nur als `Single`-Blatt, sonst
//!   `Error::AiNesting` (Details `nesting.rs`).
//! - Eine KI hat an Gruppen mit Admin-Recht und an sensiblen Gruppen gar keinen Zugriff und sieht sie nicht
//!   (jede Methode antwortet wie für eine nicht vorhandene Id, in Listen fehlen sie; Nutzerentscheidung
//!   2026-11-16, Details `nesting.rs`).
//! - Reihenfolge von `Vec`-Feldern ist nicht Vertrag, außer wo das Doc sie nennt (Konformitätstests vergleichen
//!   als Menge). Ein fallengelassenes Future bricht kein mehrschrittiges Schreiben ab (Kaskaden laufen zu Ende).
//!
//! Async (`async-trait`, wie `appointments_v1`, `packages_v1`, ADR 0070): angeboten über `groups.capnp` v1.
//! Speicherneutral: der Anbieter `LocalGroups` hält typisierte Tabellen `groups_*` in fs-objects v5 (ADR 0086,
//! PLAN.md §3). Nicht in v1: Folgen zwischen unabhängigen Gruppen (`Follows`, eigener Trait `follows_v1`, sobald
//! ein Weg zwischen Instanzen und ein angemeldeter Absender existieren; PLAN O3), Import/Export (Grenz-Adapter
//! `Exchange`, PLAN §9), Gruppenschlüssel (MLS, ADR 0067; PLAN §3.3).
//!
//! Wer implementiert: `LocalGroups` (fs-groups-core, über den bestehenden `GroupRepo`), `RemoteGroups` (Client
//! über `groups.capnp` v1), `FakeGroups`/`FailingGroups`/`SlowGroups`/`EmptyGroups` (Crate `fs-groups-testdata`).

// Gezielte Ausnahmen von clippy::pedantic (Regel der Strategie: nur mit Begründung):
// - module_name_repetitions: die Typnamen sind Vertrag (`MirrorStep` in `mirror.rs`), alles wird flach über
//   `pub use` angeboten; der Modulname ist nur Ablage.
// - missing_errors_doc: jede Methode nennt ihre Fehler im Doc-Satz, die Ursachen stehen typisiert in
//   `errors.rs`; eine `# Errors`-Rubrik je Methode wiederholte nur.
#![allow(clippy::module_name_repetitions, clippy::missing_errors_doc)]

mod api;
mod consumed;
mod errors;
mod group;
mod members;
mod mirror;
mod nesting;
mod provided;
mod rights;
pub mod shared;

pub use api::*;
pub use consumed::*;
pub use errors::*;
pub use group::*;
pub use members::*;
pub use mirror::*;
pub use nesting::*;
pub use provided::*;
pub use rights::*;
pub use shared::*;
