//! Der Trait `GroupsV2`: Eigentümer, Stufen, Voreinstellung der Vererbung, Zusätze. Ein Anbieter von v2 ist
//! auch Anbieter von v1 (`Groups`, gleicher Anbieter, gleiche Verbindung; wie `GroupMembers` und `Mirroring`); die
//! v1-Regeln gelten unverändert, mit zwei Anpassungen, die der Anbieter einhält:
//! die Rolle Eigentümer lässt sich nur über die Methoden hier vergeben, und ein Eigentümer ist nur mit `LastManager`
//! (v1-Pfad) bzw. `LastOwner` (v2-Pfad) zu entfernen.
//!
//! Pattern: Repository + Port (Trait; alle Methoden Pflicht). Wer handelt, ist die Verbindung, nie ein Parameter.

use super::{Addon, AddonKind, Intervention, Level, Result2, Scope, ScopeGrant};
use crate::groups_v1::shared::{ObjectId, PersonRef, Revision};
use crate::groups_v1::{Group, GroupDraft, Membership, NestOutcome};
use async_trait::async_trait;

/// Die Methoden von v2.
#[async_trait]
pub trait GroupsV2: Send + Sync {
    // ---- Eigentümer (ADR 0095)
    /// Gruppe anlegen; die fragende Person wird erste Eigentümerin (v1 `create` gibt nur `Manage`).
    async fn create_owned(&self, draft: GroupDraft) -> Result2<Group>;
    /// Die Eigentümer (Mitgliedschaften mit der Rolle `owner`), nur Personenverweise. `Withheld`, wenn die fragende
    /// Person die Gruppe nicht lesen darf (sensible Gruppe ohne Einblick); nie eine leere Liste dafür.
    async fn owners(&self, group: ObjectId) -> Result2<Vec<Membership>>;
    /// Die Stufe der fragenden Person in der Gruppe.
    async fn my_level(&self, group: ObjectId) -> Result2<Level>;
    /// Eine Person zur Eigentümerin ernennen (nur Eigentümer; ein bekannter Mensch, kein KI-Konto, kein Profil).
    async fn appoint_owner(&self, group: ObjectId, person: &PersonRef) -> Result2<Membership>;
    /// Eine Eigentümerin zum einfachen Mitglied herabstufen (nur Eigentümer; nicht die letzte: `LastOwner`).
    async fn demote_owner(&self, membership: ObjectId, expected: Revision) -> Result2<Membership>;
    /// Eigentum übergeben: nur die Eigentümerin selbst, gegen die Revision ihrer Mitgliedschaft; das Ziel muss direktes
    /// Mitglied sein. Zuerst wird das Ziel Eigentümer, dann (wenn `keep` falsch ist) die Abgebende herabgestuft.
    async fn transfer_ownership(
        &self,
        from: ObjectId,
        expected: Revision,
        to: &PersonRef,
        keep: bool,
    ) -> Result2<()>;
    /// Den Eigentümer eines Objekts (`PersonRef`, Person ODER Gruppe) zu Personen auflösen: eine Person ist sie selbst,
    /// eine Gruppe ist ihre Eigentümerin (Eigentum fließt über keine Kante).
    async fn resolve_owner(&self, owner: &PersonRef) -> Result2<Vec<PersonRef>>;

    // ---- Vererbung
    /// Kante mit dem `default_mode` der OBEREN Gruppe (der Modus bleibt je Kante gespeichert und änderbar).
    async fn nest_default(&self, child: ObjectId, parent: ObjectId) -> Result2<NestOutcome>;

    // ---- Stufen (a) und (b), Eingriff
    /// Die vergebenen Bereiche (sichtbar für jeden, der in einer Admin-Gruppe steht oder einen Bereich hält).
    async fn scopes(&self) -> Result2<Vec<ScopeGrant>>;
    /// Einen Bereich (`Project`/`AllGroups`) einer Admin-Gruppe vergeben; nur wer `Project` hält (der erste Bereich:
    /// die Einrichterin).
    async fn grant_scope(&self, admin_group: ObjectId, scope: Scope) -> Result2<ScopeGrant>;
    /// Einen Bereich entziehen (nur `Project`-Halter; der letzte `Project`: `LastScope`).
    async fn revoke_scope(&self, grant: ObjectId) -> Result2<()>;
    /// In eine Gruppe eingreifen: Grund Pflicht, endet nach einer Stunde, sichtbar im Protokoll der Gruppe.
    async fn open_intervention(
        &self,
        group: ObjectId,
        scope: Scope,
        reason: &str,
    ) -> Result2<Intervention>;
    /// Einen eigenen Eingriff schließen.
    async fn close_intervention(&self, intervention: ObjectId) -> Result2<()>;
    /// Die Eingriffe in eine Gruppe mit ihren Handlungen; für jede Person, die die Gruppe lesen darf (`Withheld` sonst).
    async fn interventions(&self, group: ObjectId) -> Result2<Vec<Intervention>>;

    // ---- Zusätze (Komposition)
    /// Die Zusätze einer Gruppe (`Withheld`, wenn die fragende Person die Gruppe nicht lesen darf).
    async fn addons(&self, group: ObjectId) -> Result2<Vec<Addon>>;
    /// Einen Zusatz anhängen (mindestens Verwalter; jede IRI einmal je Gruppe). Ändert nichts an Mitgliedern, Rechten
    /// oder Vererbung.
    async fn attach_addon(&self, group: ObjectId, kind: AddonKind) -> Result2<Addon>;
    /// Angaben eines Zusatzes ändern (gleiche IRI), gegen die Revision.
    async fn update_addon(
        &self,
        addon: ObjectId,
        expected: Revision,
        kind: AddonKind,
    ) -> Result2<Addon>;
    /// Einen Zusatz lösen, gegen die Revision.
    async fn detach_addon(&self, addon: ObjectId, expected: Revision) -> Result2<()>;
}
