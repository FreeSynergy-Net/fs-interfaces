//! Verschachtelung: Kante mit Richtung und Modus, Anfrage mit Zustimmung, Herkunft wirksamer Mitglieder
//! (Tabellen `groups_nesting`, `groups_nesting_request`, PLAN.md §3).
//!
//! Pattern: Value Object (Kante, Anfrage, Herkunft) + State (Lebenslauf einer Anfrage) + Specification (die
//! Regel der wirksamen Mitgliedschaft steht hier, jeder Anbieter rechnet sie gleich; Konformitätstest).
//!
//! Regel (Nutzerentscheidungen 2026-11-16, PLAN O1). Gespeichert wird die Kante: Kind (unten), Eltern (oben),
//! Modus. Daraus folgt der GERICHTETE Mitgliedsgraph, genau wie Kanidm ihn hält ("X ist Mitglied von Y"):
//! - `BottomUp`: das Kind ist Mitglied der Eltern (unten in oben). Beispiel: die Betreiberin Helfa arbeitet so
//!   (Teams in einem Bereich). Mitglieder zählen nach oben, die Rechte der oberen Gruppe gelten für alle darunter.
//! - `TopDown`: die Eltern sind Mitglied des Kindes (oben in unten, die Kante umgekehrt). Mitgliedschaft fließt
//!   nach unten, Rechte fließen nach oben: die obere Gruppe steht in `member` der unteren (Beispiel: 'Chefs'
//!   steht in `member` von 'Vizechefs'; Chefs dürfen, was die Vizechefs dürfen, und sind auch wirksame Mitglieder
//!   der Vizechefs: sie sehen deren Mitgliederliste und werden mit eingeladen). Ungleich tiefe Ebenen sind
//!   gleichgültig (Nutzerentscheidung O21, 2026-11-16: "Chefs dürfen, was die darunter dürfen").
//! - `Single` (`Einzeln`): nur Anzeige- und Ordnungskante im Baum (wer zu wem gehört), kein Bogen im
//!   Mitgliedsgraph; nichts fließt, weder Mitglieder noch Rechte. Kanidm ist dafür FLACH: die Kante wird nicht
//!   gespiegelt (Nutzerentscheidung 2026-11-16). Jede Gruppe legt ihre Mitglieder selbst fest.
//!
//! KI-Regel (Nutzerentscheidung 2026-11-16; ADR 0054, 0056; "die KI ist am gefährlichsten, wenn sie angreift"):
//! Eine KI ist ein Konto mit der Art KI (People, `PersonFacts`) und darf NUR in einer KI-Gruppe (`Group::ai`)
//! Mitglied sein, nie direkt in einer Menschengruppe; in eine KI-Gruppe kommt umgekehrt kein Mensch (Trennung
//! nach Kontoart; ein Mensch dort: `AiNesting`, unbekannter Status: `Error::AiStatusUnknown`; in
//! Menschengruppen bleiben Unbekannte erlaubt). Eine KI-Gruppe nimmt an KEINER wirksamen Kante teil: kein
//! `BottomUp`, kein `TopDown`; erlaubt ist nur `Single`, und nur als BLATT ganz unten im Baum (sie ist Kind, nie
//! Eltern): sie erbt nie Rechte und vererbt nie Rechte. Rechte bekommt eine KI-Gruppe nur ausdrücklich und
//! einzeln (Prinzip "darf am wenigsten"), `Manage` nie.
//! - BEIM SCHREIBEN ist jeder Verstoß `Error::AiNesting { group, path }`, nie stilles Weglassen: KI-Konto in
//!   einer Menschengruppe (`path` = nur die Gruppe), Mensch in einer KI-Gruppe, KI-Gruppe mit wirksamer Kante
//!   oder mit Kindern (`path` = Kind, Eltern), `update(ai)` auf einer Gruppe, die das verletzte, auch
//!   `ai: false` an einer KI-Gruppe mit KI-Mitgliedern.
//! - Ist der Zustand OHNE Schreiben falsch geworden (die Person wird nach dem Eintritt als KI markiert, oder
//!   umgekehrt eine KI als Mensch oder unbekannt in einer KI-Gruppe; Altdaten; `restore`), bleibt die
//!   Mitgliedschaft stehen, ZÄHLT aber nicht: `InForce::KindMismatch` ("Kontoart passt nicht zur Gruppe"), mit
//!   Befund. Sie fehlt in `effective_members`, `may_do` und `GroupMembers`; der Spiegel meldet `LeftOut` plus
//!   `HeldBack`, läuft weiter und plant `remove` für die KI überall, wo sie schon gespiegelt ist. Der Plan
//!   scheitert dann NICHT ganz. Eine KI-Gruppe mit wirksamer Kante aus Altdaten (die Kante hat kein `in_force`)
//!   wirkt wie `Single`: die Kante fehlt in `effective_members` und `may_do`, der Spiegel meldet `HeldBack`.
//! - Eine KI hat an Gruppen mit Admin-Recht und an sensiblen Gruppen GAR KEINEN Zugriff, auch nicht lesend
//!   (Nutzerentscheidung 2026-11-16: "ein Scammer fischt sonst nur Daten ab"): sie sieht solche Gruppen nicht
//!   einmal in Listen. Allgemeine Regel (statt einer Aufzählung von Methoden): JEDE Methode, lesend oder
//!   schreibend, die eine solche Gruppe als Id annimmt, antwortet einer fragenden KI genau wie für eine Id, die
//!   es nicht gibt (`NotFound`, bei `Option` `None`, `ParentFilter::ChildrenOf` wie eine unbekannte Gruppe: kein
//!   Unterschied zwischen leer und `NotFound`), geprüft vor jedem anderen Fehler; ebenso Ids von Mitgliedschaften,
//!   Einladungen, Anfragen, Kanten und Rechten einer solchen Gruppe (`set_role`, `remove_member`,
//!   `answer_`/`withdraw_nesting_request`, `change_nesting_mode`, `remove_nesting`, `answer_`/`revoke_invitation`,
//!   `revoke_grant`). Nie `Forbidden`: das verriete, dass es sie gibt; `may_do` gilt auch, wenn die KI nach einer
//!   fremden Person fragt. Zahlen und Filter zählen nur sichtbare Gruppen (`GroupSummary::parents`/`children`,
//!   `ParentFilter::Roots` = Gruppen ohne SICHTBAREN Elternteil, `NestingEffect`). `nesting_effect` braucht
//!   `Manage` an einer Seite. Ist eine der beiden Ids eine solche Gruppe, antwortet es einer KI wie oben
//!   `NotFound` (vor dem Rechtefehler); sonst ist es für eine KI `Forbidden`. Einträge, die
//!   eine solche Gruppe nennen, fehlen in Ergebnissen und Fehlerpfaden: Kanten, Anfragen, Einladungen, `Grant`,
//!   `HistoryKind::other`, `ChangeKind`, Befunde und Schritte des Spiegels, Pfade von `Cycle`/`AiNesting`/
//!   `ModeConflict`; in Listen fehlen die Gruppen. `Mirroring` lesen darf eine KI gar nicht (`Forbidden`, nie mit
//!   der Id einer verborgenen Gruppe). Die Antwort `Reason::ViaGroup` verbirgt eine Inhaber-Gruppe, die die
//!   Fragende nicht lesen darf, wie `Inherited::from` (`holder = None`).
//!
//! Die Regel liegt im künftigen Crate `group-nesting` mit eigenen, über die Id generischen Typen; Anbieter
//! bilden sie ab (`From`), dieser Ordner enthält keine Logik und re-exportiert nichts aus dem Crate.
//!
//! Wirksame Mitglieder von G = direkte Mitglieder von G und die direkten Mitglieder jeder Gruppe X, von der ein
//! gerichteter Weg X → … → G im Mitgliedsgraph führt (Hülle über alle Bögen, wie Kanidm `memberof`; ein Weg darf
//! `BottomUp`- und `TopDown`-Bögen mischen, so wie Kanidm es auch tut). Rechnen: eine Breitensuche rückwärts ab G,
//! O(Gruppen + Kanten + Mitgliedschaften); der erste Fund je Quelle ist der kürzeste Weg (`Inherited::via`).
//! Kreise: verboten im Mitgliedsgraph (NACH der Umkehrung der `TopDown`-Kanten, VOR jedem Spiegeln; entsteht z. B.,
//! wenn in einem Baum A in B und B in A steht) und im Baum selbst (alle Modi, auch `Single`); beides ist
//! `Error::Cycle` mit Pfad. Kanidm selbst würde einen Kreis annehmen und jede Gruppe darin zum Mitglied jeder
//! anderen machen; darum prüft fs-groups vorher. Tiefe jeder Kette höchstens `MAX_NESTING_DEPTH` KANTEN.
//!
//! Grenzfälle (Konformitätstests): Selbstkante (Kind = Eltern) = `Error::Cycle { path: [A, A] }`, in jedem Modus.
//! Gleiches Paar noch einmal, in gleichem ODER anderem Modus = `Error::AlreadyThere(id der bestehenden Kante)`:
//! nicht still idempotent, weil der Fehler die Id nennt; der Weg zu einem anderen Modus ist `change_nesting_mode`.
//! Umgekehrtes Paar (B unter A, obwohl A unter B steht) = `Error::Cycle` (Baum). Diamanten und mehrere Wege
//! zwischen zwei Gruppen sind erlaubt; jede Person zählt je Gruppe einmal. Sensibel (DSGVO Art. 9): kein Bogen
//! zwischen einer sensiblen und einer nicht sensiblen Gruppe, in BEIDE Richtungen (aus sensibel heraus würde
//! Besonderes offen, nach sensibel hinein würden Menschen ohne eigenen Beitritt wirksame Mitglieder einer
//! Art.-9-Gruppe: `Error::ModeConflict(SensitiveFlow)`); `Single` hat keinen Bogen und bleibt erlaubt.

use super::shared::{ObjectId, PersonRef, Revision, Utc};

/// Längste erlaubte Kette von Kanten (Baum, alle Modi) und von Bögen (Mitgliedsgraph); gezählt werden KANTEN
/// (16 Kanten = 17 Gruppen). Kanidm nennt keine Grenze (Quelltext `plugins/memberof.rs` v1.12.1: Schleife bis
/// keine Änderung mehr); die Grenze hält Anzeige, Rechnen und Erklärung ("geerbt aus … über …") überschaubar.
/// Tiefere Bäume, die jemand von Hand in Kanidm baut, werden nie übernommen: sie bleiben Befunde
/// (`FindingKind::ForeignNesting`). Altdaten oder `restore` über der Grenze: die Kante bleibt weg
/// (`HistoryKind::NestingRemoved`), wie beim Kreis.
pub const MAX_NESTING_DEPTH: u32 = 16;

/// Modus einer Kante (siehe Kopf des Moduls).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NestingMode {
    /// Kind ist Mitglied der Eltern (Kanidm: Kind in `member` der Eltern). Mitglieder zählen nach oben, die
    /// Rechte der oberen Gruppe gelten für alle darunter (Beispiel Helfa).
    BottomUp,
    /// Eltern sind Mitglied des Kindes (Kanidm: Eltern in `member` des Kindes). Mitgliedschaft fließt nach
    /// unten, Rechte fließen nach oben: Chefs dürfen, was die Vizechefs dürfen, und sind auch deren wirksame
    /// Mitglieder (sehen die Mitgliederliste, werden mit eingeladen).
    TopDown,
    /// Nur Baum, nichts fließt: keine Vererbung von Mitgliedern, keine von Rechten.
    Single,
}

/// Bestätigte Kante "Kind gehört zu Eltern". Eine Kante je Paar; beide Seiten haben zugestimmt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nesting {
    pub id: ObjectId,
    pub revision: Revision,
    pub child: ObjectId,
    pub parent: ObjectId,
    pub mode: NestingMode,
    pub linked: Utc,
    /// Wer die Kante angefragt (oder, mit Rechten an beiden Seiten, sofort gesetzt) hat.
    pub linked_by: PersonRef,
    /// Wer auf der anderen Seite zugestimmt hat; `None`, wenn `linked_by` beide Seiten verwaltet.
    pub approved_by: Option<PersonRef>,
}

/// Seite einer Kante.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NestingSide {
    /// Die Gruppe ist Kind (Kanten zu ihren Eltern).
    AsChild,
    /// Die Gruppe ist Eltern (Kanten zu ihren Kindern).
    AsParent,
}

/// Was angefragt wird.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NestingRequestKind {
    /// Neue Kante mit diesem Modus.
    Link { mode: NestingMode },
    /// Modus einer bestehenden Kante ändern (nur Verstärken oder Richtungswechsel; zu `Single` geht sofort).
    /// Hat die Kante beim Zustimmen nicht mehr den Modus `from`, ist die Antwort `Error::Changed`; wird die
    /// Kante gelöst oder eine Seite gelöscht, steht die offene Anfrage danach auf `Withdrawn`.
    ChangeMode {
        nesting: ObjectId,
        from: NestingMode,
        to: NestingMode,
    },
}

/// Lebenslauf einer Anfrage. Nur `Pending` lässt sich beantworten oder zurückziehen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RequestState {
    Pending,
    Approved,
    Declined,
    Withdrawn,
}

/// Anfrage an die Seite, die die fragende Person nicht verwaltet (wie eine Einladung, aber zwischen Gruppen).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NestingRequest {
    pub id: ObjectId,
    pub revision: Revision,
    pub child: ObjectId,
    pub parent: ObjectId,
    pub kind: NestingRequestKind,
    /// Die Seite, deren Verwaltung (`Right::Manage`) zustimmen muss.
    pub asked: NestingSide,
    pub state: RequestState,
    pub requested_by: PersonRef,
    pub requested: Utc,
    pub answered_by: Option<PersonRef>,
    pub answered: Option<Utc>,
}

/// Richtung einer Anfrage aus Sicht einer Gruppe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestSide {
    /// Diese Gruppe ist die gefragte Seite (sie stimmt zu oder lehnt ab).
    Incoming,
    /// Diese Gruppe ist die fragende Seite.
    Outgoing,
}

/// Antwort auf eine Anfrage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestAnswer {
    Approve,
    Decline,
}

/// Ergebnis von `request_nesting` und `change_nesting_mode`: verwaltet die fragende Person beide Seiten (oder
/// schwächt sie zu `Single` ab), gilt es sofort; sonst bleibt eine offene Anfrage an die andere Seite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NestOutcome {
    Linked(Nesting),
    Requested(NestingRequest),
}

/// Eine geerbte Herkunft: "geerbt aus X". Regel "verborgen": `from = None` heißt, die fragende Person darf X
/// nicht lesen; `via` nennt nur Gruppen, die sie lesen darf. Ist auch nur eine Gruppe auf dem Weg nicht lesbar,
/// ist `via` leer (die Form des Baums bliebe sonst nicht verborgen); die Person sieht dann nur "geerbt".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inherited {
    pub from: Option<ObjectId>,
    /// Kürzester Weg im Mitgliedsgraph, beginnt mit `from` und endet mit der gefragten Gruppe; leer, wenn
    /// eine Gruppe darauf nicht lesbar ist (siehe oben).
    pub via: Vec<ObjectId>,
}

/// Was eine Kante bewirken würde (für den Zustimmungsdialog beider Seiten), über die ganze Hülle: auch Gruppen
/// weit oberhalb oder unterhalb und Geschwister über gemischte Wege. Nur Zahlen, keine Namen: die andere Seite
/// erfährt nichts über Mitglieder, die sie nicht lesen darf.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NestingEffect {
    pub mode: NestingMode,
    /// Zahl der Gruppen, deren wirksame Mitglieder sich ändern würden (nur die Zahl, keine Ids).
    pub groups_affected: u32,
    /// Personen, die neu wirksames Mitglied mindestens einer betroffenen Gruppe würden.
    pub persons_gained: u32,
    /// Personen, die wirksame Mitgliedschaft in mindestens einer betroffenen Gruppe verlören (Moduswechsel).
    pub persons_lost: u32,
}
