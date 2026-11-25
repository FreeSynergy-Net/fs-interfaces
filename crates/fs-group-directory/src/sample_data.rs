//! The fixed names of the sample: invented people (addresses on example.org only), groups and roles.
//!
//! Pattern: none (plain constants).

/// The humans of the sample (the first one asks and created every group): 17 of the 20 persons.
pub(crate) const HUMANS: [&str; 17] = [
    "Alice Chair",
    "Bob Deputy",
    "Carol Lead",
    "Dave Outreach",
    "Erin Events",
    "Frank Stage",
    "Gina Guest",
    "Hank Union",
    "Ida Admin",
    "Jonas Intern",
    "Kira Sections",
    "Liam Projects",
    "Mara Coordinator",
    "Noah Volunteer",
    "Olga Trainee",
    "Pia Newcomer",
    "Quinn Archive",
];
/// The two AI accounts of the sample.
pub(crate) const AGENTS: [&str; 2] = ["Assistant Bot", "Scribe Bot"];
/// The sample people that ARE entries of the directory sample (`fs-person-directory`): the key the sample is asked by,
/// the number of the entry. Their reference and display name come from there; the other people are invented here.
pub(crate) const DIRECTORY_PEOPLE: [(&str, u8); 4] = [
    ("Alice Chair", 1),
    ("Bob Deputy", 10),
    ("Carol Lead", 12),
    ("Assistant Bot", 4),
];
/// The person nobody knows to be human or AI.
pub(crate) const UNKNOWN: &str = "Visitor Unknown";

/// One group of the sample: name, kind, organisation, mirrored, sensitive (a row of fixed data, so the flags stay plain).
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct GroupSpec {
    pub name: &'static str,
    pub kind: &'static str,
    pub mirror: bool,
    pub sensitive: bool,
}

const fn spec(name: &'static str, kind: &'static str, mirror: bool) -> GroupSpec {
    GroupSpec {
        name,
        kind,
        mirror,
        sensitive: false,
    }
}

/// The fifteen groups (the last one goes to the trash). The `TopDown` chain Chiefs to Newcomers has nine levels.
pub(crate) fn groups() -> [GroupSpec; 15] {
    [
        spec("Association", "Verein", true),
        spec("Outreach team", "Team", false),
        spec("Events team", "Team", false),
        spec("Chiefs", "Leitung", false),
        spec("Deputies", "Leitung", false),
        spec("Team leads", "Leitung", false),
        spec("Section heads", "Leitung", false),
        spec("Project leads", "Leitung", false),
        spec("Coordinators", "Leitung", false),
        spec("Volunteers", "Gruppe", false),
        spec("Trainees", "Gruppe", false),
        spec("Newcomers", "Gruppe", false),
        spec("Administrators", "Admin", true),
        GroupSpec {
            sensitive: true,
            ..spec("Union members", "Gruppe", false)
        },
        spec("Archive", "Gruppe", false),
    ]
}

/// The edges: child, parent, mode (`b` bottom-up, `t` top-down, `s` single). The chain Chiefs, Deputies, ...,
/// Newcomers is `TopDown` over nine levels (Chiefs sit as a Single branch under the association). Outreach team has
/// two parents (the association `BottomUp`, Deputies `TopDown`: a short side branch of the chain), Trainees two (the
/// chain `TopDown`, the Events team `BottomUp`: a mixed node, its people count up and the chiefs' people flow in).
pub(crate) const EDGES: [(&str, &str, char); 13] = [
    ("Outreach team", "Association", 'b'),
    ("Events team", "Association", 'b'),
    ("Chiefs", "Association", 's'),
    ("Deputies", "Chiefs", 't'),
    ("Team leads", "Deputies", 't'),
    ("Section heads", "Team leads", 't'),
    ("Project leads", "Section heads", 't'),
    ("Coordinators", "Project leads", 't'),
    ("Volunteers", "Coordinators", 't'),
    ("Trainees", "Volunteers", 't'),
    ("Newcomers", "Trainees", 't'),
    ("Outreach team", "Deputies", 't'),
    ("Trainees", "Events team", 'b'),
];

/// Direct members: group, person, role label (`None` = plain member).
pub(crate) const MEMBERS: [(&str, &str, Option<&str>); 18] = [
    ("Chiefs", "Alice Chair", Some("Chair")),
    ("Deputies", "Bob Deputy", None),
    ("Team leads", "Carol Lead", None),
    ("Section heads", "Kira Sections", None),
    ("Project leads", "Liam Projects", None),
    ("Coordinators", "Mara Coordinator", None),
    ("Volunteers", "Noah Volunteer", None),
    ("Trainees", "Olga Trainee", None),
    ("Newcomers", "Pia Newcomer", None),
    ("Newcomers", "Visitor Unknown", None),
    ("Association", "Alice Chair", Some("Chair")),
    ("Outreach team", "Dave Outreach", None),
    ("Events team", "Erin Events", None),
    ("Administrators", "Alice Chair", Some("Administrator")),
    ("Administrators", "Ida Admin", None),
    ("Union members", "Hank Union", None),
    ("Union members", "Ida Admin", None),
    ("Archive", "Quinn Archive", None),
];
