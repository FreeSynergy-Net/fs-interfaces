//! Spiegel in Verzeichnisse (Kanidm): fs-groups legt fest, das Verzeichnis gleicht an (ADR 0047, 0055, 0082
//! Entscheidung 6). `GroupDirectory` ist VERBRAUCHT (Anbieter: fs-identity, Kanidm-REST); `Mirroring` bietet
//! fs-groups an (Aufrufer: `fs-server`, CLI, Fenster nur lesend). Rollen, Rechte und Einladungen werden nicht
//! gespiegelt (PLAN §4). Nur Gruppen mit `mirror = true`, nie `sensitive`.
//!
//! Spiegel je Modus (PLAN O1, Nutzerentscheidung 2026-11-16; Entwurf ADR-0082-Nachtrag): NATIV, nichts flach
//! ausgerechnet. Der Mitgliedsgraph (`nesting.rs`) geht eins zu eins nach Kanidm:
//! - `BottomUp`: das Kind steht in `member` der Eltern (`DirectoryGroup::nested` der Eltern). Mitglieder zählen
//!   nach oben, die Rechte der oberen Gruppe gelten für alle darunter.
//! - `TopDown`: die Eltern stehen in `member` des Kindes (`DirectoryGroup::nested` des Kindes). Mitgliedschaft
//!   fließt nach unten, Rechte fließen nach oben: die obere Gruppe steht in `member` der unteren (Beispiel:
//!   'Chefs' steht in `member` von 'Vizechefs'; Chefs dürfen, was die Vizechefs dürfen, und sind auch wirksame
//!   Mitglieder der Vizechefs: sie sehen deren Mitgliederliste und werden mit eingeladen).
//! - `Single`: kein Bogen (Anzeigekante im Baum, Kanidm ist flach); die Gruppe trägt nur ihre direkten Mitglieder.
//!
//! KI-Regel auf der HÜLLE (`nesting.rs`): KI-Konten stehen in keiner Gruppe; beim SCHREIBEN erzwingt der Vertrag das
//! mit `Error::AiNotMember`. Ist der Zustand ohne Schreiben falsch geworden (Neumarkierung nach dem Eintritt, Altdaten: `InForce::KindMismatch`),
//! scheitert `mirror_plan` NICHT ganz: die KI bleibt draußen, der Plan meldet `LeftOut(person)` plus
//! `HeldBack(Gruppe)`, läuft weiter und plant `remove` (`Update.remove`) für die KI überall, wo sie schon
//! gespiegelt ist, auch in Admin-Gruppen; ein Bogen, der sie wirksam hineinbrächte, wird nicht geschrieben.
//! Admin-Regel auf der Hülle (ADR 0056 Punkt 6, B1 des Trait-Reviews) gilt ebenso für Personen mit UNBEKANNTEM
//! KI-Status: Kanidm gibt `memberof` einer Admin-Gruppe an alle wirksamen Mitglieder und kennt die Sperre nicht;
//! ein Bogen, der sie wirksam in eine Gruppe mit Admin-Recht brächte (auch über weitere Bögen darüber), wird
//! nicht geschrieben (Konformitätstest: `BottomUp`-Kind und `TopDown`-Obergruppe einer Admin-Gruppe). In Kanidm stehen
//! nur Menschen (ADR 0056 Punkt 8).
//!
//! Pattern: Strategy + Adapter (Verzeichnis austauschbar) + Template Method beim Anbieter (Plan, Prüfung,
//! Schreiben in Reihenfolge) + Memento (Plan-Ticket bindet das Anwenden an den gezeigten Plan).

use super::shared::{ObjectId, PersonRef, Reference, Utc};
use super::{Error, Result};
use async_trait::async_trait;

/// Kennung eines Verzeichnisses (z. B. die Kanidm-Adresse); trennt die Buchführung je Verzeichnis.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DirectoryId(pub Reference);

/// Konto im Verzeichnis, das fs-groups nicht zuordnen kann (wörtlich aus dem Verzeichnis, z. B. SPN).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DirectoryAccount(pub String);

/// Eine Gruppe, wie sie im Verzeichnis stehen soll oder steht. `members` sind Personen (die Zuordnung Person →
/// Konto liegt in fs-identity, ADR 0082 Entscheidung 6), nur die direkten; `nested` = Gruppen, die in Kanidm
/// Mitglied dieser Gruppe sind (`BottomUp`: Mitglieder zählen nach oben, die Rechte der oberen Gruppe gelten für
/// alle darunter; `TopDown`: Mitgliedschaft fließt nach unten, Rechte fließen nach oben: die obere Gruppe steht
/// in `member` der unteren, Beispiel 'Chefs' in `member` von 'Vizechefs': Chefs dürfen, was die Vizechefs
/// dürfen, und sind auch wirksame Mitglieder der Vizechefs); `foreign` nur beim Lesen: Konten oder Gruppen ohne
/// Zuordnung.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectoryGroup {
    pub group: ObjectId,
    pub name: String,
    pub members: Vec<PersonRef>,
    pub nested: Vec<ObjectId>,
    pub foreign: Vec<DirectoryAccount>,
}

/// Fehler eines Verzeichnisses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectoryError {
    Unreachable,
    Refused,
    NotAllowed,
    Corrupt,
}

/// Ergebnis von `apply`: Personen ohne Konto bleiben draußen und werden gemeldet, der Rest gilt.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Applied {
    pub unmapped: Vec<PersonRef>,
}

/// VERBRAUCHT: das Verzeichnis (Anbieter fs-identity). Idempotent; ändert nur Gruppen, die fs-groups angelegt hat.
#[async_trait]
pub trait GroupDirectory: Send + Sync {
    fn directory_id(&self) -> DirectoryId;
    async fn list(&self) -> std::result::Result<Vec<DirectoryGroup>, DirectoryError>;
    async fn read(
        &self,
        group: ObjectId,
    ) -> std::result::Result<Option<DirectoryGroup>, DirectoryError>;
    async fn apply(&self, group: &DirectoryGroup) -> std::result::Result<Applied, DirectoryError>;
    async fn remove(&self, group: ObjectId) -> std::result::Result<(), DirectoryError>;
}

/// Ein geplanter Schritt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MirrorStep {
    Create {
        group: ObjectId,
    },
    Update {
        group: ObjectId,
        add: Vec<PersonRef>,
        remove: Vec<PersonRef>,
        nest: Vec<ObjectId>,
        unnest: Vec<ObjectId>,
    },
    Remove {
        group: ObjectId,
    },
}

/// Nummer eines Befunds im Plan (für die Bestätigung je Fall).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FindingId(pub u32);

/// Was der Abgleich meldet statt still zu handeln.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FindingKind {
    /// Im Verzeichnis von Hand hinzugefügt: ein Konto oder eine fremde Kanidm-Gruppe (SPN, wörtlich aus dem
    /// Verzeichnis). Entfernen nur nach Bestätigung je Fall (Gate `human`).
    Foreign(DirectoryAccount),
    /// Person ohne Konto im Verzeichnis: bleibt draußen.
    Unmapped(PersonRef),
    /// Person mit unbekanntem KI-Status in einer Gruppe mit Admin-Recht: bleibt draußen (ADR 0056). Gilt für
    /// direkte Mitglieder UND für wirksame über Kanten: dann wird der Bogen nicht geschrieben (dazu `HeldBack`).
    /// Auch eine KI in Altdaten oder nach Neumarkierung (`InForce::KindMismatch`): sie wird nicht gespiegelt, und
    /// wo sie schon gespiegelt ist, plant der Abgleich `remove`.
    LeftOut(PersonRef),
    /// Gespeichertes nicht lesbar: dort bleibt das Verzeichnis, wie es ist.
    Unreadable(ObjectId),
    /// Schleife in Fremddaten: nie verschachtelt.
    Looped(ObjectId),
    /// Zurückgehalten: eine Gruppe darüber konnte nicht angeglichen werden, oder ihr Bogen in eine Admin-Gruppe
    /// wurde wegen `LeftOut` (KI auf der Hülle) nicht geschrieben; die Id ist die innere Gruppe des Bogens.
    HeldBack(ObjectId),
    /// `inner` wäre in Kanidm Mitglied von `outer`, wird aber nicht gespiegelt (`mirror = false`): der Bogen fehlt
    /// in Kanidm, die Hülle dort ist kleiner als `effective_members`. Nie still; der Mensch entscheidet, ob er
    /// `inner` spiegelt.
    NestedNotMirrored { outer: ObjectId, inner: ObjectId },
    /// Von Hand in Kanidm wurde `inner` in `outer` gelegt, und beide sind fs-groups-Gruppen, aber fs-groups hat
    /// keine solche Kante (auch: tiefer als `MAX_NESTING_DEPTH`, ein Kreis, oder sensibel unverträglich). Nie
    /// still `unnest` planen (O10): erst nach Bestätigung je Fall entfernt (`overwritable`). Eine fremde
    /// Kanidm-Gruppe darin ist `Foreign` (SPN).
    ForeignNesting { outer: ObjectId, inner: ObjectId },
}

/// Ein Befund; `overwritable` = darf nach Bestätigung überschrieben werden.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirrorFinding {
    pub id: FindingId,
    pub group: Option<ObjectId>,
    pub kind: FindingKind,
    pub overwritable: bool,
}

/// Bindet `mirror_apply` an genau den gezeigten Plan (wie `expected`): ändert sich etwas, ist er veraltet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirrorTicket(pub u64);

/// Trockenlauf (ADR 0082 Entscheidung 4): was geschähe, ohne etwas zu schreiben.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirrorPlan {
    pub ticket: MirrorTicket,
    pub directory: DirectoryId,
    pub made: Utc,
    pub steps: Vec<MirrorStep>,
    pub findings: Vec<MirrorFinding>,
}

/// Ausgang eines Laufs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunOutcome {
    Done,
    DoneWithFindings,
    Partial,
    Failed,
}

/// Bericht eines Laufs (in fs-groups gespeichert, Tabellen `groups_mirror_*`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirrorReport {
    pub directory: DirectoryId,
    pub started: Utc,
    pub ended: Utc,
    pub outcome: RunOutcome,
    pub applied: Vec<MirrorStep>,
    pub findings: Vec<MirrorFinding>,
    pub actor: PersonRef,
}

/// Spiegel-Zustand je Verzeichnis (für die Ansicht "Spiegel").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirrorStatus {
    pub directory: DirectoryId,
    pub last: Option<MirrorReport>,
    pub groups_mirrored: u32,
}

/// Spiegel-Zustand einer Gruppe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GroupMirrorState {
    /// `mirror = false` oder `sensitive`.
    NotMirrored,
    /// Noch nie abgeglichen.
    Pending,
    /// Beim letzten Lauf angeglichen; offene Befunde dieser Gruppe stehen dabei.
    Synced {
        at: Utc,
        findings: Vec<MirrorFinding>,
    },
    /// Beim letzten Lauf nicht angeglichen.
    Failed {
        at: Utc,
        findings: Vec<MirrorFinding>,
    },
}

/// Fehler beim Spiegeln.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MirrorError {
    Directory(DirectoryError),
    PlanStale,
    Groups(Error),
}

/// Spiegeln anbieten. Eine KI darf `Mirroring` gar nicht lesen (`nesting.rs`).
/// Wer handelt, braucht `Manage` an jeder Gruppe, die der Plan ändert (sonst bleibt sie als
/// `HeldBack` stehen); nie eine KI (ADR 0056, 0082: der Mensch führt aus).
#[async_trait]
pub trait Mirroring: Send + Sync {
    /// Trockenlauf gegen das Verzeichnis. Verletzt der Speicher die KI-Regel ohne Schreiben (Neumarkierung,
    /// Altdaten), läuft der Plan weiter: `LeftOut` plus `HeldBack`, `remove` für die KI.
    async fn mirror_plan(
        &self,
        directory: &dyn GroupDirectory,
    ) -> std::result::Result<MirrorPlan, MirrorError>;
    /// Anwenden genau dieses Plans; überschreibt `Foreign`-Befunde nur, wenn ihre Nummer in `overwrite` steht.
    async fn mirror_apply(
        &self,
        directory: &dyn GroupDirectory,
        ticket: MirrorTicket,
        overwrite: &[FindingId],
    ) -> std::result::Result<MirrorReport, MirrorError>;
    /// Letzter Stand je Verzeichnis (aus dem gespeicherten Bericht, ohne das Verzeichnis zu fragen).
    async fn mirror_status(&self) -> Result<Vec<MirrorStatus>>;
    /// Stand einer Gruppe je Verzeichnis.
    async fn group_mirror_state(
        &self,
        group: ObjectId,
    ) -> Result<Vec<(DirectoryId, GroupMirrorState)>>;
}
