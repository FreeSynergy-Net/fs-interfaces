//! Fehler der v2-Methoden: typisiert, nie ein Text. Fehler aus v1 laufen durch (`V1`).
//!
//! Pattern: Error Enum (typisierte Ursache + Zusatzfelder), `From` für die v1-Fehler.

use super::{Level, Scope};
use crate::groups_v1::Error;
use crate::groups_v1::shared::{ObjectId, PersonRef};

/// Was bei einem v2-Aufruf schiefgehen kann.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error2 {
    /// Ein Fehler aus `groups_v1` (nicht gefunden, geändert, verboten, Eingabe, ...).
    V1(Error),
    /// Die Handlung braucht die Stufe Eigentümer (ernennen, herabstufen, übergeben, löschen mit Eigentümer).
    NeedsOwner { group: ObjectId },
    /// Die Handlung braucht mindestens diese Stufe (Zusätze anhängen: Verwalter).
    NeedsLevel { group: ObjectId, level: Level },
    /// Danach hätte die Gruppe keinen Eigentümer mehr (nie automatisch ersetzt).
    LastOwner { group: ObjectId, person: PersonRef },
    /// Das Ziel einer Übergabe ist kein direktes Mitglied.
    NotDirectMember { group: ObjectId, person: PersonRef },
    /// Die Mitgliedschaft gehört nicht der handelnden Person, oder sie trägt die Rolle nicht.
    NotYourMembership(ObjectId),
    /// Die Gruppe darf die fragende Person nicht lesen: zurückgehalten, nie eine leere Liste.
    Withheld { group: ObjectId },
    /// Der Bereich ist nicht vergebbar (`Group(_)`) oder die handelnde Person hält ihn nicht.
    ScopeDenied(Scope),
    /// Der letzte Bereich `Project` lässt sich nicht entziehen.
    LastScope,
    /// Ein Eingriff braucht einen Grund.
    ReasonMissing,
    /// Der Eingriff ist unbekannt, fremd oder schon zu.
    NoSuchIntervention(ObjectId),
    /// Den Zusatz gibt es an dieser Gruppe schon (die Id des vorhandenen).
    AddonAlreadyThere(ObjectId),
    /// Den Zusatz gibt es nicht (oder er hängt an einer Gruppe, die die fragende Person nicht sieht).
    NoSuchAddon(ObjectId),
}

impl From<Error> for Error2 {
    fn from(e: Error) -> Self {
        Self::V1(e)
    }
}

/// Ergebnis eines v2-Aufrufs.
pub type Result2<T> = std::result::Result<T, Error2>;
