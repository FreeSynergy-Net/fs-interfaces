//! The example trees: a network built from below and a company built from above (invented names). What the fake says
//! about them: shape, owners, moderators, who is a member of what in which direction, the Kanidm tree of both.

use fs_group_directory::groups::{
    GroupDraft, GroupGovernance, GroupQuery, Groups, Mirroring, NestingMode, ParentFilter, Right,
    Scope, Visibility,
};
use fs_group_directory::{EXAMPLE_LOCAL_MEMBERS, FakeMirror, ROLE_TEMPLATES, example};

#[tokio::test]
async fn the_example_is_whole_and_marked_as_example_data() {
    let e = example(EXAMPLE_LOCAL_MEMBERS);
    assert!(e.problems().is_empty(), "{:#?}", e.problems());
    let local = e.id("Ortsgruppe Eschenfeld").expect("local group");
    let g = e.groups.get(local).await.expect("get").expect("group");
    assert_eq!(g.description.as_deref(), Some("Beispieldaten"));
    let members = e.groups.members(local, None, 1000).await.expect("members");
    // 200 generated members plus the owner, the founder is gone.
    assert_eq!(members.items.len(), EXAMPLE_LOCAL_MEMBERS + 1);
    let small = example(5);
    let l = small.id("Ortsgruppe Eschenfeld").expect("group");
    let m = small.groups.members(l, None, 1000).await.expect("members");
    assert_eq!(m.items.len(), 6, "the count is a parameter");
}

#[tokio::test]
async fn every_group_decides_for_itself_with_its_own_single_owner_and_the_founder_gone() {
    let e = example(10);
    for name in [
        "Weltnetz Lindenau",
        "Ortsgruppe Eschenfeld",
        "Organisationsgruppe Wasserwerk",
        "Beispiel Maschinenbau",
        "Team Testautomatisierung",
    ] {
        let g = e.id(name).expect("group");
        let owners = e.groups.owners(g).await.expect("owners");
        assert_eq!(owners.len(), 1, "{name}");
        assert_ne!(
            Some(owners[0].person.clone()),
            e.person("Gruendungsperson Beispiel")
        );
    }
}

#[tokio::test]
async fn moderators_are_a_role_template_without_authority() {
    let e = example(10);
    let t = &ROLE_TEMPLATES[0];
    assert!(t.term_months == 12 && !t.instructs && !t.admin);
    let role = e.role("Moderator").expect("template role");
    let local = e.id("Ortsgruppe Eschenfeld").expect("group");
    let page = e.groups.members(local, None, 100).await.expect("members");
    let mods: Vec<_> = page
        .items
        .iter()
        .filter(|m| m.role.as_ref() == Some(&role))
        .collect();
    assert_eq!(mods.len(), 2);
    for m in mods {
        assert!(
            !e.groups
                .may_do(local, &m.person, Right::Manage)
                .await
                .expect("may_do")
                .allowed,
            "a moderator may not manage"
        );
    }
}

#[tokio::test]
async fn the_network_flows_upward_the_company_downward_and_the_advisory_group_not_at_all() {
    let e = example(10);
    let local = e.id("Ortsgruppe Eschenfeld").expect("group");
    let world = e.id("Weltnetz Lindenau").expect("group");
    let (local_owner, world_owner) = (
        e.person("Mara Hellwig").expect("p"),
        e.person("Ilka Brandt").expect("p"),
    );
    // Bottom-up: the local member is an effective member of the global group, the global owner is not of the local.
    let on_world = e
        .groups
        .effective_members(world, None, 1000)
        .await
        .expect("e")
        .items;
    assert!(on_world.iter().any(|m| m.person == local_owner));
    let on_local = e
        .groups
        .effective_members(local, None, 1000)
        .await
        .expect("e")
        .items;
    assert!(!on_local.iter().any(|m| m.person == world_owner));
    // Unequal depth: the second local group reaches the top over two edges, the first over three.
    let w_owner = e.person("Joris Keller").expect("p");
    assert!(on_world.iter().any(|m| m.person == w_owner));
    // Top-down: the head of the company is an effective member of the deepest team, not the other way.
    let firm = e.id("Beispiel Maschinenbau").expect("group");
    let deep = e.id("Team Testautomatisierung").expect("group");
    let (head, lead) = (
        e.person("Karla Sommer").expect("p"),
        e.person("Hanna Moll").expect("p"),
    );
    let on_deep = e
        .groups
        .effective_members(deep, None, 1000)
        .await
        .expect("e")
        .items;
    assert!(on_deep.iter().any(|m| m.person == head));
    let on_firm = e
        .groups
        .effective_members(firm, None, 1000)
        .await
        .expect("e")
        .items;
    assert!(!on_firm.iter().any(|m| m.person == lead));
    // The advisory group (Single edge): its owner is no effective member above, nobody above is one in it.
    let advisory = e.id("Organisationsgruppe Wasserwerk").expect("group");
    let region = e.id("Region Birkental").expect("group");
    let adv_owner = e.person("Bente Carstens").expect("p");
    let on_region = e
        .groups
        .effective_members(region, None, 1000)
        .await
        .expect("e")
        .items;
    assert!(!on_region.iter().any(|m| m.person == adv_owner));
    let on_adv = e
        .groups
        .effective_members(advisory, None, 1000)
        .await
        .expect("e")
        .items;
    assert!(on_adv.len() == 1, "only its own owner");
    // Ownership flows over no edge.
    assert!(
        !e.groups
            .owners(local)
            .await
            .expect("owners")
            .iter()
            .any(|m| m.person == world_owner)
    );
}

#[tokio::test]
async fn both_trees_give_the_member_tree_in_the_directory_with_the_company_reversed() {
    let e = example(10);
    // Every group manages itself and the founder is gone from all of them, so the setup person (the installer) runs
    // the mirror the open way: a Project scope for an admin group of hers, then an intervention with a reason per group.
    let founder = e.person("Gruendungsperson Beispiel").expect("installer");
    e.groups.set_asker(founder);
    let admins = e
        .groups
        .create(GroupDraft {
            name: "Projektverwaltung Beispiel".to_owned(),
            kind: None,
            description: None,
            default_mode: NestingMode::Single,
            visibility: Visibility::Everyone,
            sensitive: false,
            mirror: false,
        })
        .await
        .expect("admin group");
    e.groups
        .grant_scope(admins.id, Scope::Project)
        .await
        .expect("first Project scope");
    let all = e
        .groups
        .list(
            &GroupQuery {
                text: String::new(),
                parent: ParentFilter::Any,
                member: None,
                include_deleted: false,
            },
            None,
            100,
        )
        .await
        .expect("list")
        .items;
    for g in all.iter().map(|s| s.id).filter(|g| *g != admins.id) {
        e.groups
            .open_intervention(g, Scope::Project, "Beispieldaten: Spiegel-Lauf")
            .await
            .expect("intervention");
    }
    let mirror = FakeMirror::new();
    let plan = e.groups.mirror_plan(&mirror).await.expect("plan");
    let report = e
        .groups
        .mirror_apply(&mirror, plan.ticket, &[])
        .await
        .expect("apply");
    assert!(report.findings.is_empty(), "{:#?}", report.findings);
    let nested = |name: &str| {
        let mut v = mirror
            .group(e.id(name).expect("group"))
            .map(|d| d.nested)
            .unwrap_or_default();
        v.sort();
        v
    };
    let ids = |names: &[&str]| {
        let mut v: Vec<_> = names.iter().map(|n| e.id(n).expect("group")).collect();
        v.sort();
        v
    };
    // The network as it is (the upper group holds the lower), the advisory group not.
    assert_eq!(
        nested("Region Birkental"),
        ids(&["Ortsgruppe Eschenfeld", "Themengruppe Gemuesegarten"])
    );
    assert_eq!(nested("Ueberregion Nordmark"), ids(&["Region Birkental"]));
    // The company reversed (the lower group holds the upper), so both trees are one direction for Kanidm.
    assert_eq!(nested("Team Testautomatisierung"), ids(&["Team Software"]));
    assert_eq!(nested("Team Software"), ids(&["Abteilung Entwicklung"]));
    assert_eq!(
        nested("Abteilung Entwicklung"),
        ids(&["Beispiel Maschinenbau"])
    );
    assert_eq!(
        nested("Beispiel Maschinenbau"),
        [] as [fs_group_directory::groups::ObjectId; 0]
    );
}
