//! The fixed sample: its shape (fifteen groups, twenty persons, three modes, every case of the plan) and what the
//! fake says about it.

use fs_group_directory::groups_v1::shared::PersonArt;
use fs_group_directory::groups_v1::{
    FindingKind, GroupMembers, GroupQuery, Groups, InvitationState, MirrorStep, Mirroring,
    NestingMode, NestingSide, ParentFilter, PersonDirectory, RequestSide, RequestState,
};
use fs_group_directory::{group_ref, sample};

fn all() -> GroupQuery {
    GroupQuery {
        text: String::new(),
        parent: ParentFilter::Any,
        member: None,
        include_deleted: true,
    }
}

#[tokio::test]
async fn the_sample_is_whole_and_has_fifteen_groups_one_of_them_deleted() {
    let s = sample();
    assert!(s.problems().is_empty(), "{:#?}", s.problems());
    let page = s.groups.list(&all(), None, 100).await.expect("list");
    assert_eq!(page.items.len(), 15);
    assert_eq!(
        page.items
            .iter()
            .filter(|g| g.restorable_until.is_some())
            .count(),
        1
    );
    let live = s
        .groups
        .list(
            &GroupQuery {
                include_deleted: false,
                ..all()
            },
            None,
            100,
        )
        .await
        .expect("list");
    assert_eq!(live.items.len(), 14);
}

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
async fn the_sample_uses_all_three_modes_in_one_tree() {
    let s = sample();
    let mut modes = Vec::new();
    for name in CHAIN
        .iter()
        .chain(&["Association", "Events team", "Outreach team"])
    {
        let id = s.id(name).expect("group");
        for e in s
            .groups
            .nestings(id, NestingSide::AsChild)
            .await
            .expect("nestings")
        {
            modes.push(e.mode);
        }
    }
    for m in [
        NestingMode::BottomUp,
        NestingMode::TopDown,
        NestingMode::Single,
    ] {
        assert!(modes.contains(&m), "{m:?} missing");
    }
    let assoc = s.id("Association").expect("group");
    let top = s
        .groups
        .effective_members(assoc, None, 50)
        .await
        .expect("members")
        .items;
    let olga = s.person("Olga Trainee").expect("person");
    let counts_up = top
        .iter()
        .find(|m| m.person == olga)
        .expect("Trainees count up to the association");
    let trainees = s.id("Trainees").expect("group");
    let events = s.id("Events team").expect("group");
    assert!(
        counts_up
            .inherited
            .iter()
            .any(|i| i.via == vec![trainees, events, assoc]),
        "two BottomUp edges: Trainees, Events team, Association"
    );
}

#[tokio::test]
async fn the_sample_has_no_ai_group_an_admin_group_a_sensitive_group_and_every_invitation_state() {
    let s = sample();
    let q = s.groups.list(&all(), None, 100).await.expect("list").items;
    assert!(q.iter().all(|g| !g.ai), "there are no AI groups");
    assert!(q.iter().any(|g| g.sensitive && g.name == "Union members"));
    let events = s.id("Events team").expect("group");
    let outreach = s.id("Outreach team").expect("group");
    let mut states: Vec<InvitationState> = s
        .groups
        .invitations(events)
        .await
        .expect("invitations")
        .iter()
        .map(|i| i.state)
        .collect();
    states.extend(
        s.groups
            .invitations(outreach)
            .await
            .expect("invitations")
            .iter()
            .map(|i| i.state),
    );
    for st in [
        InvitationState::Pending,
        InvitationState::Accepted,
        InvitationState::Declined,
        InvitationState::Expired,
    ] {
        assert!(states.contains(&st), "{st:?} missing");
    }
    let open = s
        .groups
        .nesting_requests(outreach, RequestSide::Incoming)
        .await;
    // Carol manages Events and asks; Alice manages Outreach, which is the asked side.
    assert!(
        open.expect("requests")
            .iter()
            .any(|r| r.state == RequestState::Pending)
    );
}

#[tokio::test]
async fn the_sample_knows_only_invented_people_on_example_org() {
    let s = sample();
    for name in ["Alice Chair", "Bob Deputy", "Gina Guest"] {
        let p = s.person(name).expect("person");
        assert_eq!(p.art, PersonArt::Person);
        let (shown, address) = s.people.display(&p).await.expect("display").expect("known");
        assert_eq!(shown, name);
        assert!(
            address.is_some_and(|a| a.ends_with("@example.org")),
            "{name}"
        );
    }
}

#[tokio::test]
async fn the_sample_mirror_plan_shows_a_foreign_account_and_removes_the_trashed_group() {
    let s = sample();
    let plan = s.groups.mirror_plan(&s.mirror).await.expect("plan");
    assert!(plan.findings.iter().any(
        |f| matches!(&f.kind, FindingKind::Foreign(a) if a.0.ends_with("example.org"))
            && f.overwritable
    ));
    let archive = s.id("Archive").expect("group");
    assert!(
        plan.steps
            .iter()
            .any(|x| matches!(x, MirrorStep::Remove { group } if *group == archive))
    );
}

#[tokio::test]
async fn the_narrow_view_resolves_groups_for_other_programs() {
    let s = sample();
    let events = group_ref(s.id("Events team").expect("group"));
    let card = s
        .groups
        .describe(&events)
        .await
        .expect("describe")
        .expect("card");
    assert_eq!(card.name, "Events team");
    assert!(card.effective_members.is_some_and(|n| n >= 2));
    let members = s.groups.members_of(&events).await.expect("members");
    assert!(members.contains(&s.person("Erin Events").expect("person")));
    let mine = s
        .groups
        .memberships_of(&s.person("Olga Trainee").expect("person"))
        .await
        .expect("groups");
    assert!(
        mine.contains(&events),
        "Olga is in Trainees, below Events team (BottomUp)"
    );
}
