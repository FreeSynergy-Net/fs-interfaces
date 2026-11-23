//! Der Fehler des Themas.
//!
//! Pattern: Value Object (Fehlerart als reine Daten; der Satz für Menschen ist Sache des Programms, ADR 0057).

use super::{ObjectId, Problem, Revision};

/// Fehler des Themas. Eine unbekannte oder hier nicht gültige Zone ist KEIN eigener Fehler (Typ-Tausch,
/// Entscheidung 2a): sie ist `Invalid` mit einem [`Problem`] am Feld (`zone`, `view_zone`, `changes.zone` …) und
/// dem Code der Zonenprüfung aus `fs_types::TypeProblem` (heute `zone.bad`). Die Form prüft `fs_types::Zone`,
/// ob eine IANA-Zone existiert, sagt die tz-Datenbank (fs-jscalendar `time::Zone::parse`); nie raten (ADR 0085).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Der Anbieter kann es ausdrücklich nicht (ADR 0045), mit Fähigkeitskennung; `capabilities` sagt es vorher.
    Unsupported { capability: String, reason: String },
    /// Vergleich-und-Setzen gescheitert: jemand hat vorher geschrieben.
    Changed { current: Revision },
    NotFound(ObjectId),
    /// Nicht erlaubt (z. B. fremder Kalender, den man nur lesen darf), mit Fähigkeitskennung. Nicht
    /// `Unsupported`: der Anbieter kann es, die fragende Person darf es nicht.
    Forbidden { capability: String },
    /// Der Kalender ist schreibgeschützt (Abo, [`super::CalendarSource::Subscribed`]): niemand schreibt dort Termine;
    /// was drinsteht, bestimmt die Quelle beim nächsten Abruf. Nicht `Forbidden` (keine Frage der Person).
    ReadOnly { calendar: ObjectId },
    /// Prüfung gescheitert; nichts geschrieben. Je Feld ein [`Problem`] mit Code (auch die Zone, siehe oben).
    Invalid(Vec<Problem>),
    /// Speicher oder Anbieter nicht erreichbar.
    Unavailable(String),
}
/// Ergebnis jeder Methode des Themas.
pub type Result<T> = std::result::Result<T, Error>;
