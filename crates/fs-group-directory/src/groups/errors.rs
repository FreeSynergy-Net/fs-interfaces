//! Typisierte Fehler des Ordners `groups/`. Jede Ursache ist eine Variante mit typisierten Zusatzfeldern, nie
//! ein Text. `Display` dient nur Entwicklern (englisch); die Person sieht Übersetzungen über die Kennungen
//! (ADR 0057).
//!
//! Pattern: Error Enum (typisierte Ursache + Zusatzfelder), `Display` nur zur Anzeige.

use super::shared::{ObjectId, PersonArt, PersonRef, Reference, Revision, TypeProblem, Utc};
use super::{InvitationState, Level, RequestState, Right, Scope};
use std::fmt;

/// Wovon etwas nicht gefunden wurde.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Missing {
    Group(ObjectId),
    Membership(ObjectId),
    Invitation(ObjectId),
    Nesting(ObjectId),
    NestingRequest(ObjectId),
    Grant(ObjectId),
    /// Der Verweis zeigt auf nichts, das People kennt (oder auf eine gelöschte Person, AS2 `Tombstone`).
    Person(Reference),
}

/// Feld einer Eingabe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Kind,
    Description,
    Person,
    Role,
    Holder,
    Expires,
    Mirror,
    Abzug,
}

/// Was an einem Feld falsch ist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProblemKind {
    Empty,
    TooLong {
        max: u32,
    },
    /// Steuer- oder unsichtbare Zeichen (Schreibweise wie heute `text.rs`).
    BadCharacters,
    BadReference(TypeProblem),
    /// Zeitpunkt liegt in der Vergangenheit.
    InPast,
    /// `mirror` bei `sensitive`.
    SensitiveNotMirrored,
    /// `abzug` ist in diesem Thema nie erlaubt.
    NoAbzug,
}

/// Ein Prüffehler an einem Feld.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldProblem {
    pub field: Field,
    pub problem: ProblemKind,
}

/// Warum etwas gerade nicht erreichbar ist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unreachable {
    Store,
    Server,
    People,
}

/// Art der Speicherstörung (wie `std::io::ErrorKind`, ohne Text).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageKind {
    NotFound,
    PermissionDenied,
    NoSpace,
    Corrupt,
    Locked,
    Encrypted,
    Other,
}

/// Warum ein Modus (oder `sensitive`) nicht passt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConflictKind {
    /// Ein Bogen des Mitgliedsgraphen (`from` wäre Mitglied von `to`) verbände eine sensible mit einer nicht
    /// sensiblen Gruppe, in EINER der beiden Richtungen (DSGVO Art. 9): sensibel nach nicht sensibel machte
    /// Besonderes offen, nicht sensibel nach sensibel machte Menschen ohne eigenen Beitritt zu wirksamen
    /// Mitgliedern. Gilt für Kanten, Moduswechsel, `restore` und `update(sensitive)`; `Single` ist erlaubt.
    SensitiveFlow { from: ObjectId, to: ObjectId },
}

/// Fehler des Themas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    NotFound(Missing),
    /// Vergleich-und-Setzen gescheitert: jemand hat vorher geschrieben.
    Changed {
        current: Revision,
    },
    /// Die fragende Person hat dieses Recht an der Gruppe nicht (`Read` auch für `Visibility`).
    Forbidden {
        group: ObjectId,
        right: Right,
    },
    /// Die Kante schlösse einen Kreis im Mitgliedsgraphen (nach Umkehrung der `TopDown`-Kanten) oder im Baum
    /// (alle Modi, auch `Single`); `path` nennt ihn, erste Gruppe = letzte Gruppe.
    Cycle {
        path: Vec<ObjectId>,
    },
    /// Eine Kette (Baum oder Mitgliedsgraph) würde länger als `MAX_NESTING_DEPTH`.
    TooDeep {
        max: u32,
    },
    /// Modus oder Schutz passen nicht zusammen; nichts geschrieben. Nie `Forbidden`: das ist ein Rechtefehler.
    ModeConflict(ConflictKind),
    /// Ein KI-Konto ist nie Mitglied einer Gruppe (ADR 0091, 0095: KI nur über den Bot-Platz). `group` = die Gruppe,
    /// an der es scheitert. Nichts geschrieben, nie still.
    AiNotMember { group: ObjectId },
    /// Falsche Art im Verweis: eine Gruppe als Mitglied (dafür `request_nesting`), eine Organisation als
    /// Eingeladene usw.
    WrongArt {
        field: Field,
        got: PersonArt,
    },
    /// Die fragende Person ist nicht die eingeladene.
    NotInvitee,
    /// Die Einladung ist abgelaufen (beim Antworten als `Expired` gespeichert).
    Expired {
        at: Utc,
    },
    /// Einladung nicht mehr offen (angenommen, abgelehnt, widerrufen, abgelaufen).
    InvitationClosed(InvitationState),
    /// Verschachtelungs-Anfrage nicht mehr offen.
    RequestClosed(RequestState),
    /// Kante, offene Anfrage, Modus oder Mitgliedschaft gibt es schon in dieser Form (trägt die bestehende Id).
    AlreadyThere(ObjectId),
    /// Admin-Rolle oder `Manage` für eine als KI markierte Person (ADR 0056 Punkt 6).
    AgentNotAdmin,
    /// Admin-Rolle, `Manage` oder Mitgliedschaft, aber People sagt nicht, ob die Person ein Mensch ist (im Zweifel nein).
    AiStatusUnknown,
    /// `restore` nach Ablauf der Frist oder für eine nicht gelöschte Gruppe.
    NotRestorable(ObjectId),
    /// Die letzte Person mit `Manage` (oder der Installierende) kann sich nicht selbst entfernen; die Gruppe
    /// bliebe ohne Verwaltung.
    LastManager,
    /// Prüfung gescheitert; nichts geschrieben.
    Invalid(Vec<FieldProblem>),
    Unavailable(Unreachable),
    Storage(StorageKind),
    // ---- Eigentümer, Stufen, Eingriff, Zusätze (ADR 0095)
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
pub type Result<T> = std::result::Result<T, Error>;

impl From<TypeProblem> for Error {
    fn from(p: TypeProblem) -> Self {
        Self::Invalid(vec![FieldProblem {
            field: Field::Person,
            problem: ProblemKind::BadReference(p),
        }])
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(m) => write!(f, "not found: {m:?}"),
            Self::Changed { current } => {
                write!(f, "changed meanwhile (now revision {})", current.0)
            }
            Self::Forbidden { right, .. } => write!(f, "not allowed: needs {right:?}"),
            Self::Cycle { path } => write!(f, "would close a cycle of {} groups", path.len()),
            Self::TooDeep { max } => write!(f, "nesting deeper than {max} levels"),
            Self::WrongArt { field, got } => write!(f, "{field:?} may not be a {got:?}"),
            Self::Expired { at } => write!(f, "invitation expired at {} ms", at.millis()),
            Self::Invalid(p) => write!(f, "invalid input: {p:?}"),
            other => write!(f, "{other:?}"),
        }
    }
}
impl std::error::Error for Error {}
