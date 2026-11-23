//! The shape of the sample that PLAN section 6 asks for: the `TopDown` chain of nine levels, several parents, twenty
//! persons, the two open requests.

use fs_group_directory::groups::{
    Groups, NestingMode, NestingRequestKind, NestingSide, PersonFacts, RequestSide, RequestState,
};
use fs_group_directory::sample;

const CHAIN: [&str; 9] = [
    "Chiefs",
    "Deputies",
    "Team leads",
    "Section heads",
    "Project leads",
    "Coordinators",
    "Volunteers",
    "Trainees",
    "Newcomers",
];

#[tokio::test]
async fn the_topdown_chain_has_nine_levels_and_chiefs_are_members_of_every_level() {
    let s = sample();
    let alice = s.person("Alice Chair").expect("person");
    let ids: Vec<_> = CHAIN.iter().map(|n| s.id(n).expect("group")).collect();
    for (level, id) in ids.iter().enumerate().skip(1) {
        let eff = s
            .groups
            .effective_members(*id, None, 50)
            .await
            .expect("members")
            .items;
        let a = eff
            .iter()
            .find(|m| m.person == alice)
            .expect("Chiefs are members of the lower level");
        // Alice also founded every group (she owns each one), so she comes in over several ways: look at the one from Chiefs.
        let from_chiefs = a
            .inherited
            .iter()
            .find(|i| i.from == Some(ids[0]))
            .expect("inherited from Chiefs");
        assert_eq!(from_chiefs.via, ids[..=level].to_vec(), "level {level}");
    }
    let last = s.id("Newcomers").expect("group");
    let top = s
        .groups
        .effective_members(ids[0], None, 50)
        .await
        .expect("members")
        .items;
    assert!(
        top.iter()
            .all(|m| m.person != s.person("Pia Newcomer").expect("person")),
        "nothing flows up the chain"
    );
    assert_eq!(
        s.groups.ancestors(last).await.expect("ancestors").len(),
        10,
        "eight chain levels above, the association and the Events team"
    );
}

#[tokio::test]
async fn the_sample_has_groups_with_several_parents_and_one_shorter_side_branch() {
    let s = sample();
    let outreach = s.id("Outreach team").expect("group");
    let parents = s
        .groups
        .nestings(outreach, NestingSide::AsChild)
        .await
        .expect("nestings");
    let of = |n: &str| s.id(n).expect("group");
    assert!(
        parents
            .iter()
            .any(|e| e.parent == of("Association") && e.mode == NestingMode::BottomUp)
    );
    assert!(
        parents
            .iter()
            .any(|e| e.parent == of("Deputies") && e.mode == NestingMode::TopDown)
    );
    let trainees = s
        .groups
        .nestings(of("Trainees"), NestingSide::AsChild)
        .await
        .expect("nestings");
    assert_eq!(trainees.len(), 2, "the chain and the Events team");
    let eff = s
        .groups
        .effective_members(outreach, None, 50)
        .await
        .expect("members")
        .items;
    let alice = s.person("Alice Chair").expect("person");
    let via = &eff
        .iter()
        .find(|m| m.person == alice)
        .expect("Chiefs flow into Outreach")
        .inherited
        .iter()
        .find(|i| i.from == Some(of("Chiefs")))
        .expect("inherited from Chiefs")
        .via;
    assert_eq!(
        via.len(),
        3,
        "Chiefs, Deputies, Outreach: a shorter side branch"
    );
}

#[tokio::test]
async fn the_sample_has_twenty_persons_with_two_ai_accounts_and_one_unknown() {
    let s = sample();
    let names = [
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
        "Assistant Bot",
        "Scribe Bot",
        "Visitor Unknown",
    ];
    let (mut ai, mut unknown) = (0, 0);
    for n in names {
        let p = s.person(n).expect("person");
        match s.people.is_agent(&p).await.expect("status") {
            Some(true) => ai += 1,
            None => unknown += 1,
            Some(false) => {}
        }
    }
    assert_eq!((ai, unknown), (2, 1));
}

#[tokio::test]
async fn the_sample_keeps_one_open_edge_request_and_one_open_mode_change_request() {
    let s = sample();
    let (events, outreach) = (
        s.id("Events team").expect("group"),
        s.id("Outreach team").expect("group"),
    );
    let trainees = s.id("Trainees").expect("group");
    let edge_req = s
        .groups
        .nesting_requests(outreach, RequestSide::Incoming)
        .await
        .expect("requests");
    assert!(
        edge_req.iter().any(|r| r.state == RequestState::Pending
            && matches!(r.kind, NestingRequestKind::Link { .. }))
    );
    let change = s
        .groups
        .nesting_requests(trainees, RequestSide::Incoming)
        .await
        .expect("requests");
    let r = change
        .iter()
        .find(|r| r.state == RequestState::Pending)
        .expect("open mode change");
    assert!(
        matches!(
            r.kind,
            NestingRequestKind::ChangeMode {
                from: NestingMode::BottomUp,
                to: NestingMode::TopDown,
                ..
            }
        ) && r.parent == events
    );
}
