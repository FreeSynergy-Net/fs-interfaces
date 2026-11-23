//! Der Trait `Groups` im Ordner `groups/`: Gruppen, Verschachtelung, Mitglieder, Einladungen, Rechte.
//!
//! Pattern: Repository + Port (Trait; alle Methoden Pflicht, kein Null Object).

use super::shared::{ObjectId, Page, PersonRef, Place, Reference, Revision, Utc};
use super::{
    Change, Decision, Deleted, EffectiveGroup, EffectiveMember, Grant, Group, GroupDraft,
    GroupPatch, GroupQuery, GroupSummary, HistoryEntry, Invitation, InvitationAnswer,
    InvitationOutcome, Membership, NestOutcome, Nesting, NestingEffect, NestingMode,
    NestingRequest, NestingSide, RequestAnswer, RequestSide, Result, Right, RoleRef,
};
use async_trait::async_trait;

/// Fähigkeitskennungen (Wortlaut für Manifest, Katalog `provides` und `groups.capnp`, ADR 0068). Ein Anbieter
/// bietet alle; was die fragende Person nicht darf, ist `Error::Forbidden` (außer: KI und
/// Admin-/sensible Gruppe, dann `NotFound`, `nesting.rs`). `group.members` = Trait
/// `GroupMembers` (Kalender und Tasks finden den Anbieter darüber), `mirror.read` = Trockenlauf, Stand und
/// Bericht von `Mirroring`, `mirror.write` = `mirror_apply`.
pub const CAPABILITIES: [&str; 12] = [
    "group.read",
    "group.write",
    "group.nest",
    "member.read",
    "member.write",
    "invite.write",
    "invite.answer",
    "right.read",
    "right.write",
    "group.members",
    "mirror.read",
    "mirror.write",
];

/// Die Schnittstelle von `FreeSynergy` Groups. Wer handelt, ist die fragende Person des Anbieters.
#[async_trait]
pub trait Groups: Send + Sync {
    // ---- Gruppen (group.read, group.write)
    /// Seitenweise, nach Name sortiert (Unicode-Sortierung des Anbieters, stabil). Gelöschte nur mit
    /// `include_deleted`, erkennbar an `restorable_until`. Für eine fragende KI fehlen Admin- und sensible Gruppen.
    async fn list(
        &self,
        query: &GroupQuery,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<GroupSummary>>;
    /// Eine gelöschte Gruppe ergibt `None`; ebenso eine Admin- oder sensible Gruppe für eine fragende KI.
    async fn get(&self, id: ObjectId) -> Result<Option<Group>>;
    /// Über den Verweis (`urn:uuid:…`), z. B. aus einem `PersonRef { art: Group }` des Kalenders.
    async fn get_by_reference(&self, reference: &Reference) -> Result<Option<Group>>;
    /// Die einzige Art, eine Gruppe anzulegen (ADR 0095 Pkt. 6): die fragende Person wird erste Eigentümerin (direkte
    /// Mitgliedschaft mit der Rolle `owner`). Sie bekommt KEIN eigenes, dauerhaftes `Manage`; verliert sie die Rolle
    /// (Übergabe, Herabstufen, Austritt), verliert sie jedes Recht an der Gruppe. Ist sie eine KI: `AgentNotAdmin`,
    /// nichts geschrieben.
    async fn create(&self, draft: GroupDraft) -> Result<Group>;
    async fn update(&self, id: ObjectId, expected: Revision, patch: GroupPatch) -> Result<Group>;
    /// In den Papierkorb, mit Mitgliedschaften, Einladungen, Rechten, Kanten und offenen Anfragen (Kinder
    /// bleiben, verlieren diese Eltern). Spiegel: beim nächsten Abgleich entfernt.
    async fn delete(&self, id: ObjectId, expected: Revision) -> Result<Deleted>;
    /// Bis `restorable_until`; holt alles zurück, was `delete` mitnahm. Eine Kante, die inzwischen einen Kreis
    /// schlösse (oder zu tief, oder sensibel unverträglich wäre, ), bleibt weg und
    /// steht als `NestingRemoved` im Protokoll; Mitgliedschaften, die die KI-Regel inzwischen verletzen (Person
    /// inzwischen als KI markiert), kommen zurück und zählen nicht
    /// (`InForce::KindMismatch`); offene Anfragen von `delete` sind `Withdrawn` und kommen nicht zurück.
    async fn restore(&self, id: ObjectId) -> Result<Group>;

    // ---- Verschachtelung (group.nest; Regel und Modi: `nesting.rs`)
    /// Kante "Kind gehört zu Eltern" mit Modus. `BottomUp`: Mitglieder zählen nach oben, die Rechte der oberen
    /// Gruppe gelten für alle darunter. `TopDown`: Mitgliedschaft fließt nach unten, Rechte fließen nach oben
    /// (Chefs in `member` von Vizechefs: Chefs dürfen, was die Vizechefs dürfen, und sind auch deren wirksame
    /// Mitglieder). Die fragende Person braucht `Manage` an mindestens einer Seite; an beiden: `Linked`, sonst
    /// `Requested` an die andere Seite. Geprüft (auch beim Zustimmen erneut): `Cycle` (auch Selbstkante und
    /// umgekehrtes Paar), `TooDeep`, `ModeConflict` (sensibel, beide Richtungen), `Single` ist nur Anzeige, nicht gespiegelt; schon
    /// da: `AlreadyThere` (Kante in gleichem oder anderem Modus, oder offene Anfrage; trägt die Id).
    async fn request_nesting(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Result<NestOutcome>;
    /// Was die Kante (oder ein Moduswechsel auf `mode`) bewirken würde, ohne zu schreiben; gleiche Prüfungen.
    /// `Manage` an einer Seite nötig, also für eine KI nie (`NotFound`, wenn eine Seite eine Admin-/sensible
    /// Gruppe ist, `nesting.rs`; sonst `Forbidden`); Zahlen nur über sichtbare Gruppen.
    async fn nesting_effect(
        &self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Result<NestingEffect>;
    /// Bestätigte Kanten der Gruppe auf einer Seite (Baumansicht mit Modus je Kante).
    async fn nestings(&self, group: ObjectId, side: NestingSide) -> Result<Vec<Nesting>>;
    /// Anfragen, an denen die Gruppe beteiligt ist, älteste zuerst (`Manage` an der Gruppe nötig).
    async fn nesting_requests(
        &self,
        group: ObjectId,
        side: RequestSide,
    ) -> Result<Vec<NestingRequest>>;
    /// Nur wer die gefragte Seite verwaltet. Eine KI nie (nach der Sichtbarkeitsregel, `nesting.rs`):
    /// `Forbidden { right: Manage }`, weil `Manage` für sie
    /// nie gilt (ADR 0056 Punkt 6). Zustimmen setzt die Kante bzw. den Modus und prüft alles erneut; hat sich die
    /// Kante seit der Anfrage geändert (`ChangeMode.from` stimmt nicht mehr), folgt `Changed`.
    async fn answer_nesting_request(
        &self,
        request: ObjectId,
        expected: Revision,
        answer: RequestAnswer,
    ) -> Result<NestingRequest>;
    /// Wer die Anfrage stellte oder die fragende Seite verwaltet.
    async fn withdraw_nesting_request(
        &self,
        request: ObjectId,
        expected: Revision,
    ) -> Result<NestingRequest>;
    /// Modus einer Kante ändern. Zu `Single` (Abschwächen) genügt `Manage` an einer Seite und gilt sofort;
    /// Verstärken oder Richtungswechsel braucht beide Seiten wie `request_nesting`, mit den gleichen Prüfungen
    /// . Gleicher Modus: `AlreadyThere`.
    async fn change_nesting_mode(
        &self,
        nesting: ObjectId,
        expected: Revision,
        mode: NestingMode,
    ) -> Result<NestOutcome>;
    /// Kante lösen: `Manage` an einer Seite genügt (Austritt geht immer). Offene Anfragen dieser Kante stehen
    /// danach auf `Withdrawn`.
    async fn remove_nesting(&self, nesting: ObjectId, expected: Revision) -> Result<()>;
    /// Alle Vorfahren (alle Modi), Breitensuche, jede Gruppe einmal; endet auch auf Fremddaten mit Schleife.
    /// Nur Gruppen, die die fragende Person lesen darf (sensible und verborgene fehlen, auch auf dem Weg; für
    /// eine KI auch Admin-Gruppen).
    async fn ancestors(&self, id: ObjectId) -> Result<Vec<ObjectId>>;
    /// Alle Nachkommen, wie `ancestors` (gleicher Sichtbarkeitsfilter).
    async fn descendants(&self, id: ObjectId) -> Result<Vec<ObjectId>>;

    // ---- Mitglieder (member.read, member.write)
    /// Direkte Mitglieder, seitenweise; `Visibility` gilt.
    async fn members(
        &self,
        group: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<Membership>>;
    /// Wirksame Mitglieder (Regel `nesting.rs`: direkt und über die Hülle des Mitgliedsgraphen), je Person einmal
    /// mit Herkunft "direkt"/"geerbt aus X", seitenweise in stabiler Reihenfolge des Anbieters; `Visibility` gilt.
    async fn effective_members(
        &self,
        group: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<EffectiveMember>>;
    /// `AddMembers` nötig. Gleiche Person mit gleicher Rolle: `AlreadyThere`. Gruppe als Person: `WrongArt`.
    /// KI-Konto: `AiNotMember { group }`; Person mit unbekanntem KI-Status mit Admin-Rolle: `AiStatusUnknown`.
    /// Gilt auch für `invite` und Annehmen. Wird eine Person erst später als KI markiert, bleibt die
    /// Mitgliedschaft stehen und zählt nicht (`InForce::KindMismatch`, `nesting.rs`; auch die umgekehrte).
    async fn add_member(
        &self,
        group: ObjectId,
        person: &PersonRef,
        role: Option<&RoleRef>,
    ) -> Result<Membership>;
    /// Rolle ändern (`Manage` nötig; Admin-Rolle für KI: `AgentNotAdmin`).
    async fn set_role(
        &self,
        membership: ObjectId,
        expected: Revision,
        role: Option<&RoleRef>,
    ) -> Result<Membership>;
    /// `RemoveMembers` nötig, oder die Person entfernt sich selbst (Austritt). `LastManager` möglich.
    async fn remove_member(&self, membership: ObjectId, expected: Revision) -> Result<()>;
    /// Gruppen, in denen `person` wirksam Mitglied ist, mit Herkunft (nur, was die fragende Person sehen darf;
    /// `Inherited` nach der Regel "verborgen").
    async fn groups_of(&self, person: &PersonRef) -> Result<Vec<EffectiveGroup>>;

    // ---- Einladungen (invite.write, invite.answer)
    /// Einladungen einer Gruppe, neueste zuerst (`Read` nötig).
    async fn invitations(&self, group: ObjectId) -> Result<Vec<Invitation>>;
    /// Offene Einladungen an die fragende Person, neueste zuerst.
    async fn my_invitations(&self) -> Result<Vec<Invitation>>;
    /// `AddMembers` nötig; `expires` in der Zukunft. Schon Mitglied mit dieser Rolle: `AlreadyThere`.
    async fn invite(
        &self,
        group: ObjectId,
        invitee: &PersonRef,
        role: Option<&RoleRef>,
        expires: Option<Utc>,
    ) -> Result<Invitation>;
    /// Nur die eingeladene Person. Annehmen legt die Mitgliedschaft an (ein Schritt, atomar).
    async fn answer_invitation(
        &self,
        invitation: ObjectId,
        expected: Revision,
        answer: InvitationAnswer,
    ) -> Result<InvitationOutcome>;
    /// `RemoveMembers` nötig, oder wer eingeladen hat.
    async fn revoke_invitation(
        &self,
        invitation: ObjectId,
        expected: Revision,
    ) -> Result<Invitation>;

    // ---- Rechte (right.read, right.write)
    /// Vergebene Rechte einer Gruppe (`Read` nötig).
    async fn grants(&self, group: ObjectId) -> Result<Vec<Grant>>;
    /// `Manage` nötig. Gleiches Recht an gleichen Inhaber: `AlreadyThere`. `Manage` an KI: `AgentNotAdmin`;
    /// an einer Gruppe mit Admin-Recht oder einer sensiblen Gruppe jedes Recht an eine KI: `AgentNotAdmin`.
    async fn grant(&self, group: ObjectId, holder: &PersonRef, right: Right) -> Result<Grant>;
    /// `Manage` nötig; `LastManager` möglich.
    async fn revoke_grant(&self, grant: ObjectId, expected: Revision) -> Result<()>;
    /// Auskunft für eine beliebige Person, mit Grund; im Zweifel nein. Schreibt nichts.
    async fn may_do(&self, group: ObjectId, who: &PersonRef, right: Right) -> Result<Decision>;
    /// Rechte der fragenden Person an der Gruppe (das Frontend zeigt nur, was geht).
    async fn my_rights(&self, group: ObjectId) -> Result<Vec<Right>>;

    // ---- Beobachten und Protokoll (group.read)
    /// Änderungen seit `since` (`None` = ab jetzt), älteste zuerst; nur, was die fragende Person sehen darf.
    async fn changes(&self, since: Option<&Place>, size: usize) -> Result<Page<Change>>;
    /// Protokoll einer Gruppe, älteste zuerst (`Read` nötig).
    async fn history(
        &self,
        group: ObjectId,
        after: Option<&Place>,
        size: usize,
    ) -> Result<Page<HistoryEntry>>;
}
