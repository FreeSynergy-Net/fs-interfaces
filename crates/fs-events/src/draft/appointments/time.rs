//! Zeit und Herkunft eines Termins (Herkunft aus fs-jscalendar).
//!
//! Pattern: Value Object (Zeit und Herkunft eines Termins).

use super::{Date, LocalDateTime, Zone};

/// Länge nach `JSCalendar` `Duration`: Tage sind nominal (ein "Tag" über die Zeitumstellung bleibt ein Tag).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span { pub days: u32, pub seconds: u32 }

/// Wann ein Termin liegt. Genau eine der drei Formen (ADR 0085 Abschnitt 2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum When {
    /// Mit Uhrzeit: lokale Zeit in `zone`. `end_zone` nur, wenn das Ende in einer anderen Zone liegt
    /// (Flug). `zone_assumed`: Zone wurde an der Grenze für eine schwebende Zeit eingesetzt (ADR 0085).
    Timed { start: LocalDateTime, zone: Zone, length: Span, end_zone: Option<Zone>, zone_assumed: bool },
    /// Überall dieselbe Wanduhrzeit (RFC 8984 1.4.5): keine Zone, nie `zone_assumed`. KEIN Zeitpunkt; wird nie
    /// umgerechnet und in der Zone des Betrachters gezeigt. Entsteht durch Menschen, durch die eigene Marke
    /// oder über den Kanal `Hub`, nie durch eine Zeit ohne Zone allein (Konformitätstest, siehe Dateikopf).
    Floating { start: LocalDateTime, length: Span },
    /// Ganztägig: erster Tag und Zahl der Tage (>= 1). Keine Zone, nie umgerechnet.
    AllDay { first: Date, days: u32 },
}

// Herkunft eines importierten Objekts (fs-objects `object_origin`, nur lesend) und ihr Kanal kommen aus fs-jscalendar
// (Typ-Tausch; formgleich mit dem früheren Stand-in, auch für fs-tasks). "Unverändert seit Import" wird berechnet:
// `revision == rev_at_import`; der Dateiname zählt nie (ADR 0085 Abschnitt 1). Kanal `Subscription`: `source` ist die
// IRI des Abo-KALENDERS, nie die (womöglich geheime) Feed-Adresse (`Subscription`). JMAP gibt es bewusst nicht in v1.
pub use fs_jscalendar::origin::{Origin, OriginChannel};
