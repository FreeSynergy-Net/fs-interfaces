//! The data of the example trees (fixture tables): invented names only, nothing of a real network, company or place.
//! Every row is plain data; no rule of any operator is in the code, the fake only applies the tables.
//!
//! Pattern: Fixture tables (static data applied by the builder in `example.rs`).

/// Marks every group, person and role of the example as example data (also `provider = fake`).
pub const EXAMPLE_MARK: &str = "Beispieldaten";

/// The persons doing the setup; removed from every group at the end of the build.
pub(crate) const FOUNDER: &str = "Gruendungsperson Beispiel";

/// One group: name, kind (a data word, also an addon IRI), the owner, the legal entity of the addon if there is one.
pub(crate) struct Node {
    pub name: &'static str,
    pub kind: &'static str,
    pub owner: &'static str,
    pub legal: Option<(&'static str, &'static str)>,
}

const fn node(name: &'static str, kind: &'static str, owner: &'static str) -> Node {
    Node {
        name,
        kind,
        owner,
        legal: None,
    }
}

/// A role as a TEMPLATE: data, not code. `term_months`: how long a holder serves (the fake does not end terms);
/// `instructs`: whether the role may give orders to others.
pub struct RoleTemplate {
    pub label: &'static str,
    pub admin: bool,
    pub term_months: u32,
    pub instructs: bool,
}

/// Elected yearly, no authority to instruct, no admin right.
pub const ROLE_TEMPLATES: [RoleTemplate; 1] = [RoleTemplate {
    label: "Moderator",
    admin: false,
    term_months: 12,
    instructs: false,
}];

/// Tree 1, built from below: a local group is the seed; levels (local, regional, supraregional, global) are edges
/// above it, of unequal depth. Groups of organisations sit at level two or above and only advise (a `Single` edge).
pub(crate) fn network_nodes() -> [Node; 10] {
    [
        node("Weltnetz Lindenau", "global", "Ilka Brandt"),
        node("Ueberregion Nordmark", "supraregional", "Tomke Jansen"),
        node("Region Birkental", "regional", "Gero Lund"),
        node("Region Seeland", "regional", "Wiebke Haas"),
        node("Ortsgruppe Eschenfeld", "local", "Mara Hellwig"),
        node("Ortsgruppe Wiesengrund", "local", "Joris Keller"),
        node("Themengruppe Gemuesegarten", "thematic", "Fenna Ohlsen"),
        node("Berufsgruppe Tischlerei", "professional", "Arne Petersen"),
        node("Akademie Weidenhain", "academy", "Lenja Voss"),
        node(
            "Organisationsgruppe Wasserwerk",
            "organization",
            "Bente Carstens",
        ),
    ]
}

/// (child, parent, mode `b`ottom-up, `t`op-down, `s`ingle). Depth three on one branch, two on the other.
pub(crate) const NETWORK_EDGES: [(&str, &str, char); 9] = [
    ("Ortsgruppe Eschenfeld", "Region Birkental", 'b'),
    ("Region Birkental", "Ueberregion Nordmark", 'b'),
    ("Ueberregion Nordmark", "Weltnetz Lindenau", 'b'),
    ("Ortsgruppe Wiesengrund", "Region Seeland", 'b'),
    ("Region Seeland", "Weltnetz Lindenau", 'b'),
    ("Themengruppe Gemuesegarten", "Region Birkental", 'b'),
    ("Berufsgruppe Tischlerei", "Region Seeland", 'b'),
    ("Akademie Weidenhain", "Weltnetz Lindenau", 'b'),
    ("Organisationsgruppe Wasserwerk", "Region Birkental", 's'),
];

/// Tree 2, built from above: board, departments, teams; the top group carries the addon "formal organisation".
pub(crate) fn company_nodes() -> [Node; 8] {
    [
        Node {
            legal: Some(("Beispiel Maschinenbau GmbH", "GmbH")),
            ..node("Beispiel Maschinenbau", "company", "Karla Sommer")
        },
        node("Abteilung Entwicklung", "department", "Ulf Meyer"),
        node("Abteilung Vertrieb", "department", "Pia Roth"),
        node("Abteilung Verwaltung", "department", "Nils Franke"),
        node("Team Mechanik", "team", "Sanne Ruf"),
        node("Team Software", "team", "Ole Bachmann"),
        node("Team Testautomatisierung", "team", "Hanna Moll"),
        node("Team Aussendienst", "team", "Jonte Reiss"),
    ]
}

/// Top-down: the upper group is the PARENT; members and rights flow the other way than in the first tree. Depth
/// three on one branch, two on the other.
pub(crate) const COMPANY_EDGES: [(&str, &str, char); 7] = [
    ("Abteilung Entwicklung", "Beispiel Maschinenbau", 't'),
    ("Abteilung Vertrieb", "Beispiel Maschinenbau", 't'),
    ("Abteilung Verwaltung", "Beispiel Maschinenbau", 't'),
    ("Team Mechanik", "Abteilung Entwicklung", 't'),
    ("Team Software", "Abteilung Entwicklung", 't'),
    ("Team Testautomatisierung", "Team Software", 't'),
    ("Team Aussendienst", "Abteilung Vertrieb", 't'),
];

/// Given and family names for the generated members of a group (invented).
pub(crate) const GIVEN: [&str; 10] = [
    "Anke", "Bjarne", "Caro", "Dörte", "Eike", "Finja", "Gesa", "Hauke", "Imke", "Jan",
];
pub(crate) const FAMILY: [&str; 25] = [
    "Ahlers",
    "Behrens",
    "Clasen",
    "Detlefs",
    "Eggers",
    "Fedders",
    "Gerdes",
    "Harms",
    "Ihnen",
    "Janssen",
    "Kruse",
    "Lührs",
    "Martens",
    "Nommensen",
    "Oltmanns",
    "Poppe",
    "Quedens",
    "Rickers",
    "Siemens",
    "Tamm",
    "Uphoff",
    "Vosgerau",
    "Wulf",
    "Zerr",
    "Albers",
];
