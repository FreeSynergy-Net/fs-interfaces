//! Example data in two shapes of the same contract: a network built from below (local group as the seed, levels as
//! edges above, advisory groups on a `Single` edge) and a company built from above (board, departments, teams, with
//! the addon "formal organisation"). Invented names only; every group says `Beispieldaten` in its description.
//!
//! Pattern: Builder (fixture tables from `example_data.rs`, applied through the state operations; a failed step is
//! noted in `problems`, which a test asserts empty). Nothing of the tables is a rule in the code.

use crate::example_data::{
    COMPANY_EDGES, EXAMPLE_MARK, FAMILY, FOUNDER, GIVEN, NETWORK_EDGES, Node, ROLE_TEMPLATES,
    company_nodes, network_nodes,
};
use crate::fake::FakeGroups;
use crate::groups::shared::{BasicValue as _, ObjectId, PersonRef, Reference};
use crate::groups::{AddonKind, FormalOrganization, GroupDraft, NestingMode, RoleRef, Visibility};
use crate::people::FakePeople;
use crate::sample::{Book, check};
use crate::state::State;
use std::sync::Arc;

/// Members of the first local group when no count is given.
pub const EXAMPLE_LOCAL_MEMBERS: usize = 200;

/// The example: the fake (asked by the owner of the top of the network), the consumed side, ids by name.
pub struct Example {
    /// The provider (`provider = fake`: example data, nothing real).
    pub groups: FakeGroups,
    /// The consumed people side.
    pub people: Arc<FakePeople>,
    book: Book,
}

impl Example {
    /// The id of a group by its name.
    #[must_use]
    pub fn id(&self, name: &str) -> Option<ObjectId> {
        self.book.id(name)
    }
    /// A person by name.
    #[must_use]
    pub fn person(&self, name: &str) -> Option<PersonRef> {
        self.book.person(name)
    }
    /// A role template by label (`Moderator`).
    #[must_use]
    pub fn role(&self, label: &str) -> Option<RoleRef> {
        self.book.roles.get(label).cloned()
    }
    /// Steps of the build that failed (empty when whole).
    #[must_use]
    pub fn problems(&self) -> &[String] {
        &self.book.problems
    }
}

fn mode_of(c: char) -> NestingMode {
    match c {
        'b' => NestingMode::BottomUp,
        't' => NestingMode::TopDown,
        _ => NestingMode::Single,
    }
}

fn iri(kind: &str) -> AddonKind {
    let text = format!("https://example.org/ns/group-kinds/{kind}");
    AddonKind::Other(Reference::parse(&text).expect("an IRI"))
}

/// The fixed names of the generated members of the first local group (`index` from 0).
fn member_name(index: usize) -> String {
    format!(
        "{} {} Nr. {index:03}",
        GIVEN[index % GIVEN.len()],
        FAMILY[(index / GIVEN.len()) % FAMILY.len()]
    )
}

fn make_groups(s: &mut State, out: &mut Book, nodes: &[Node]) {
    for n in nodes {
        let draft = GroupDraft {
            name: n.name.to_owned(),
            kind: Some(n.kind.to_owned()),
            description: Some(EXAMPLE_MARK.to_owned()),
            default_mode: NestingMode::BottomUp,
            visibility: Visibility::Everyone,
            sensitive: false,
            mirror: true,
        };
        let Some(g) = check(s.create(draft), n.name, &mut out.problems) else {
            continue;
        };
        out.ids.insert(n.name, g.id);
        check(s.attach_addon(g.id, iri(n.kind)), n.name, &mut out.problems);
        if let Some((legal_name, form)) = n.legal {
            let legal = AddonKind::FormalOrganization(FormalOrganization {
                legal_name: legal_name.to_owned(),
                legal_form: Some(form.to_owned()),
                registration: None,
            });
            check(s.attach_addon(g.id, legal), n.name, &mut out.problems);
        }
    }
}

fn link_edges(s: &mut State, out: &mut Book, edges: &[(&'static str, &'static str, char)]) {
    for (child, parent, mode) in edges {
        let (Some(c), Some(p)) = (out.id(child), out.id(parent)) else {
            continue;
        };
        check(
            s.request_nesting(c, p, mode_of(*mode)),
            child,
            &mut out.problems,
        );
    }
}

/// The owner named in the table becomes the owner and the founder leaves: the group decides for itself afterwards.
fn hand_over(s: &mut State, out: &mut Book, founder: &PersonRef, nodes: &[Node]) {
    for n in nodes {
        let (Some(g), Some(owner)) = (out.id(n.name), out.person(n.owner)) else {
            continue;
        };
        s.asker = founder.clone();
        check(s.add_member(g, &owner, None), n.owner, &mut out.problems);
        let mine = s
            .owner_memberships(g)
            .into_iter()
            .find(|m| &m.person == founder);
        if let Some(m) = mine {
            check(
                s.transfer_ownership(m.id, m.revision, &owner, false),
                n.name,
                &mut out.problems,
            );
        }
        s.asker = owner;
        let rows: Vec<_> = s
            .members
            .iter()
            .filter(|m| m.group == g && &m.person == founder)
            .map(|m| (m.id, m.revision))
            .collect();
        for (id, rev) in rows {
            check(s.remove_member(id, rev), n.name, &mut out.problems);
        }
    }
}

/// The example with `local_members` generated members in the first local group (200 in the use case).
#[must_use]
pub fn example(local_members: usize) -> Example {
    let people = Arc::new(FakePeople::new());
    let mut book = Book::default();
    let founder = people.human(FOUNDER);
    let (network, company) = (network_nodes(), company_nodes());
    for n in network.iter().chain(company.iter()) {
        book.persons.insert(n.owner, people.human(n.owner));
    }
    for t in &ROLE_TEMPLATES {
        book.roles
            .insert(t.label, people.add_role(t.label, t.admin));
    }
    book.persons.insert(FOUNDER, founder.clone());
    let groups = FakeGroups::with_people(Arc::clone(&people), founder.clone())
        .with_installer(founder.clone());
    {
        let mut s = groups.lock();
        make_groups(&mut s, &mut book, &network);
        make_groups(&mut s, &mut book, &company);
        link_edges(&mut s, &mut book, &NETWORK_EDGES);
        link_edges(&mut s, &mut book, &COMPANY_EDGES);
        add_local_members(&mut s, &mut book, &people, local_members);
        hand_over(&mut s, &mut book, &founder, &network);
        hand_over(&mut s, &mut book, &founder, &company);
        if let Some(top) = book.person(network[0].owner) {
            s.asker = top;
        }
    }
    Example {
        groups,
        people,
        book,
    }
}

/// Generated people in the first local group; the first two serve as moderators (the role template).
fn add_local_members(s: &mut State, out: &mut Book, people: &FakePeople, count: usize) {
    let Some(g) = out.id("Ortsgruppe Eschenfeld") else {
        return;
    };
    let moderator = out.roles.get("Moderator").cloned();
    for i in 0..count {
        let p = people.human(&member_name(i));
        let role = if i < 2 { moderator.as_ref() } else { None };
        check(s.add_member(g, &p, role), "local member", &mut out.problems);
    }
}
