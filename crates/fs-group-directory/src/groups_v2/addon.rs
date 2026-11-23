//! Zusätze einer Gruppe, per Komposition (Nutzer-Entscheid 23.11.2026): die Gruppe ist der Kern, ein Zusatz wird
//! angehängt. Kein Typ-Baum ("Organisation ist eine Gruppe"), mehrere Zusätze je Gruppe, offen erweiterbar wie
//! Rollen-Overlays: ein unbekannter Zusatz ist eine IRI, kein neuer Vertrag.
//!
//! Pattern: Composition (Facette, die an einem Kern hängt) + Value Object; nichts hängt von einem Zusatz ab.

use crate::groups_v1::shared::{BasicValue as _, ObjectId, PersonRef, Reference, Revision, Utc};

/// `org:FormalOrganization` (W3C ORG): Vokabular der juristischen Person (Verein, Firma).
pub const ORG_FORMAL_ORGANIZATION: &str = "http://www.w3.org/ns/org#FormalOrganization";

/// Angaben der juristischen Person (der Zusatz `org:FormalOrganization`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormalOrganization {
    /// Rechtlicher Name (`org:name`/`rov:legalName`), Pflicht.
    pub legal_name: String,
    /// Rechtsform in Worten der Betreiberin (`Verein`, `GmbH`), offen.
    pub legal_form: Option<String>,
    /// Registernummer (Vereinsregister, Handelsregister), offen.
    pub registration: Option<String>,
}

/// Welcher Zusatz.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AddonKind {
    /// Juristische Person (`org:FormalOrganization`).
    FormalOrganization(FormalOrganization),
    /// Ein weiterer Zusatz, benannt durch seine IRI (offen erweiterbar); trägt hier nur die Marke.
    Other(Reference),
}

impl AddonKind {
    /// Die IRI des Zusatzes; je Gruppe gibt es jede IRI höchstens einmal.
    ///
    /// # Panics
    /// Nie: die IRI von `org:FormalOrganization` ist eine gültige Referenz.
    #[must_use]
    pub fn iri(&self) -> Reference {
        match self {
            Self::FormalOrganization(_) => {
                Reference::parse(ORG_FORMAL_ORGANIZATION).expect("a W3C IRI is a reference")
            }
            Self::Other(iri) => iri.clone(),
        }
    }
}

/// Ein angehängter Zusatz.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Addon {
    pub id: ObjectId,
    pub revision: Revision,
    /// Die Gruppe, an der er hängt.
    pub group: ObjectId,
    pub kind: AddonKind,
    /// Wer ihn angehängt hat (die Verbindung, nie ein Argument).
    pub attached_by: PersonRef,
    pub attached: Utc,
}
