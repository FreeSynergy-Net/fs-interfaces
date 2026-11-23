//! Der Trait `GroupGovernance`: Eigentümer, Stufen, Voreinstellung der Vererbung, Zusätze. Derselbe Anbieter trägt
//! auch `Groups` (gleiche Verbindung; wie `GroupMembers` und `Mirroring`). Zwei Regeln hält der Anbieter ein: die Rolle
//! Eigentümer lässt sich nur über die Methoden hier vergeben, und ein Eigentümer ist nur mit `LastManager`
//! (Weg über `remove_member`) bzw. `LastOwner` (Weg über `demote_owner`) zu entfernen.
//!
//! Pattern: Repository + Port (Trait; alle Methoden Pflicht). Wer handelt, ist die Verbindung, nie ein Parameter.

use super::{Addon, AddonKind, Intervention, Level, Result, Scope, ScopeGrant};
use crate::groups::shared::{ObjectId, PersonRef, Revision};
use crate::groups::{Group, GroupDraft, Membership, NestOutcome};
use async_trait::async_trait;

/// Die Methoden der Verwaltung (Eigentümer, Stufen, Zusätze).
#[async_trait]
pub trait GroupGovernance: Send + Sync {
    // ---- Eigentümer (ADR 0095)
    /// Gruppe anlegen; die fragende Person wird erste Eigentümerin (`create` gibt nur `Manage`).
    async fn create_owned(&self, draft: GroupDraft) -> Result<Group>;
    /// Die Eigentümer (Mitgliedschaften mit der Rolle `owner`), nur Personenverweise. `Withheld`, wenn die fragende
    /// Person die Gruppe nicht lesen darf (sensible Gruppe ohne Einblick); nie eine leere Liste dafür.
    async fn owners(&self, group: ObjectId) -> Result<Vec<Membership>>;
    /// Die Stufe der fragenden Person in der Gruppe.
    async fn my_level(&self, group: ObjectId) -> Result<Level>;
    /// Eine Person zur Eigentümerin ernennen (nur Eigentümer; ein bekannter Mensch, kein KI-Konto, kein Profil).
    async fn appoint_owner(&self, group: ObjectId, person: &PersonRef) -> Result<Membership>;
    /// Eine Eigentümerin zum einfachen Mitglied herabstufen (nur Eigentümer; nicht die letzte: `LastOwner`).
    async fn demote_owner(&self, membership: ObjectId, expected: Revision) -> Result<Membership>;
    /// Eigentum übergeben: nur die Eigentümerin selbst, gegen die Revision ihrer Mitgliedschaft; das Ziel muss direktes
    /// Mitglied sein. Zuerst wird das Ziel Eigentümer, dann (wenn `keep` falsch ist) die Abgebende herabgestuft.
    async fn transfer_ownership(
        &self,
        from: ObjectId,
        expected: Revision,
        to: &PersonRef,
        keep: bool,
    ) -> Result<()>;
    /// Den Eigentümer eines Objekts (`PersonRef`, Person ODER Gruppe) zu Personen auflösen: eine Person ist sie selbst,
    /// eine Gruppe ist ihre Eigentümerin (Eigentum fließt über keine Kante).
    async fn resolve_owner(&self, owner: &PersonRef) -> Result<Vec<PersonRef>>;

    // ---- Vererbung
    /// Kante mit dem `default_mode` der OBEREN Gruppe (der Modus bleibt je Kante gespeichert und änderbar).
    async fn nest_default(&self, child: ObjectId, parent: ObjectId) -> Result<NestOutcome>;

    // ---- Stufen (a) und (b), Eingriff
    /// Die vergebenen Bereiche (sichtbar für jeden, der in einer Admin-Gruppe steht oder einen Bereich hält).
    async fn scopes(&self) -> Result<Vec<ScopeGrant>>;
    /// Einen Bereich (`Project`/`AllGroups`) einer Admin-Gruppe vergeben; nur wer `Project` hält (der erste Bereich:
    /// die Einrichterin).
    async fn grant_scope(&self, admin_group: ObjectId, scope: Scope) -> Result<ScopeGrant>;
    /// Einen Bereich entziehen (nur `Project`-Halter; der letzte `Project`: `LastScope`).
    async fn revoke_scope(&self, grant: ObjectId) -> Result<()>;
    /// In eine Gruppe eingreifen: Grund Pflicht, endet nach einer Stunde, sichtbar im Protokoll der Gruppe.
    async fn open_intervention(
        &self,
        group: ObjectId,
        scope: Scope,
        reason: &str,
    ) -> Result<Intervention>;
    /// Einen eigenen Eingriff schließen.
    async fn close_intervention(&self, intervention: ObjectId) -> Result<()>;
    /// Die Eingriffe in eine Gruppe mit ihren Handlungen; für jede Person, die die Gruppe lesen darf (`Withheld` sonst).
    async fn interventions(&self, group: ObjectId) -> Result<Vec<Intervention>>;

    // ---- Zusätze (Komposition)
    /// Die Zusätze einer Gruppe (`Withheld`, wenn die fragende Person die Gruppe nicht lesen darf).
    async fn addons(&self, group: ObjectId) -> Result<Vec<Addon>>;
    /// Einen Zusatz anhängen (mindestens Verwalter; jede IRI einmal je Gruppe). Ändert nichts an Mitgliedern, Rechten
    /// oder Vererbung.
    async fn attach_addon(&self, group: ObjectId, kind: AddonKind) -> Result<Addon>;
    /// Angaben eines Zusatzes ändern (gleiche IRI), gegen die Revision.
    async fn update_addon(
        &self,
        addon: ObjectId,
        expected: Revision,
        kind: AddonKind,
    ) -> Result<Addon>;
    /// Einen Zusatz lösen, gegen die Revision.
    async fn detach_addon(&self, addon: ObjectId, expected: Revision) -> Result<()>;
}
