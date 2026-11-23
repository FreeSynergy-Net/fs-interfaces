//! Mitgliedschaften, Rollen, Einladungen (Tabellen `groups_membership`, `groups_invitation`, PLAN.md §3).
//!
//! Pattern: Value Object (Mitgliedschaft, Einladung) + State (Lebenslauf einer Einladung, eine Übergangsfunktion
//! beim Anbieter).

use super::shared::{ObjectId, PersonRef, Reference, Revision, Utc};
use super::Inherited;

/// Verweis auf eine Rollendefinition (SKOS / W3C `org:Role`, ADR 0052). Die Definition und ihr Anzeigename
/// liegen in fs-people (`RoleBook`); fs-groups prüft nur, dass es ein `Reference` ist und ob sie Admin-Rolle ist.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RoleRef(pub Reference);

/// Warum ein Eintrag gerade nichts zählt (er bleibt stehen, ADR 0056 Punkt 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InForce {
    Yes,
    /// Kontoart passt nicht zur Gruppe oder zum Recht; die Mitgliedschaft bleibt stehen, zählt aber nicht (fehlt
    /// in `effective_members`, `may_do`, `GroupMembers`). Fälle: Admin-Rolle oder `Manage` bei einer als KI
    /// markierten Person; eine KI in einer Menschengruppe, ein Mensch oder Unbekannter in einer KI-Gruppe, wenn
    /// das ohne Schreiben entstand (Neumarkierung in BEIDE Richtungen, Altdaten, `restore`; KI-Regel in
    /// `nesting.rs`); ein Befund meldet es. Eine KI hat an Admin- und sensiblen Gruppen gar keinen Zugriff.
    KindMismatch,
    /// Admin-Rolle oder `Manage` bei einer Person, von der niemand weiß, ob sie ein Mensch ist.
    AiStatusUnknown,
}

/// Direkte Mitgliedschaft (AS2 `Relationship` in `org:Membership`-Form). `person.art` ist `Person` oder
/// `Profile`; eine Gruppe ist nie Mitglied (Verschachtelung über `request_nesting`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Membership {
    pub id: ObjectId,
    pub revision: Revision,
    pub group: ObjectId,
    pub person: PersonRef,
    /// `None` = einfaches Mitglied ohne besondere Rolle.
    pub role: Option<RoleRef>,
    pub since: Utc,
    pub added_by: PersonRef,
    /// Gesetzt, wenn die Mitgliedschaft durch Annahme einer Einladung entstand.
    pub from_invitation: Option<ObjectId>,
    pub in_force: InForce,
}

/// Wirksames Mitglied (Regel: `nesting.rs`): jede Person einmal, mit ihrer Herkunft. `direct` = Id der direkten
/// Mitgliedschaft in der gefragten Gruppe; `inherited` = je Quellgruppe eine Herkunft ("geerbt aus X"), kürzester
/// Weg zuerst. Mindestens eines von beiden ist gesetzt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveMember {
    pub person: PersonRef,
    pub direct: Option<ObjectId>,
    pub inherited: Vec<Inherited>,
}

/// Gruppe, in der eine Person wirksam Mitglied ist, mit Herkunft wie `EffectiveMember`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveGroup {
    pub group: ObjectId,
    pub direct: Option<ObjectId>,
    pub inherited: Vec<Inherited>,
}

/// Lebenslauf einer Einladung (AS2 `Invite`). Nur `Pending` lässt sich beantworten oder widerrufen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InvitationState {
    Pending,
    Accepted,
    Declined,
    Revoked,
    Expired,
}

/// Einladung an eine Person. Wer ein Mensch ohne Eintrag ist, bekommt vorher über
/// `PersonDirectory::propose` einen vorgeschlagenen Eintrag (ADR 0058, 0085); Zustellung per Mail ist ein
/// Grenz-Adapter (fs-messages), nicht dieser Trait.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invitation {
    pub id: ObjectId,
    pub revision: Revision,
    pub group: ObjectId,
    pub invitee: PersonRef,
    pub role: Option<RoleRef>,
    pub state: InvitationState,
    /// `None` = läuft nicht ab. Abgelaufene Einladungen liest man als `Expired` (der Anbieter räumt vor dem Lesen).
    pub expires: Option<Utc>,
    pub invited_by: PersonRef,
    pub sent: Utc,
    pub answered: Option<Utc>,
}

/// Antwort der eingeladenen Person (nur sie selbst, also die fragende Person; sonst `Error::NotInvitee`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvitationAnswer {
    Accept,
    Decline,
}

/// Ergebnis einer Antwort.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InvitationOutcome {
    /// Angenommen: die neue (oder schon bestehende, gleiche Rolle) Mitgliedschaft.
    Joined(Membership),
    /// Abgelehnt: die Einladung im Zustand `Declined`.
    Declined(Invitation),
}
