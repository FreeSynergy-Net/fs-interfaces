//! Example of `groups`: a group that manages itself, with addons attached by composition.
//!
//! Pattern: Fake behind the locked traits (the example talks to `FakeGroups` through `Groups` and `GroupGovernance` only).
//!
//! Run: `cargo run -p fs-group-directory --example self_managed`. The story: Alice founds the club "Musterstadt
//! Chor". She is its first owner and appoints Bob as second owner. The club is a plain group first; later it gets
//! the addon "formal organisation" (a registered association) and another one (tax-exempt), by its OWN owners:
//! nobody above is asked (level c, self-management). An addon changes nothing else: members and rights are the
//! same with and without. A stranger and a plain member cannot attach one. A document owned by the club resolves
//! to the club's owners. The club nests under a federation with the federation's `default_mode`; the mode stays
//! per edge. Finally a project administrator steps in with a reason: visible to everyone who may read the club.

use fs_group_directory::groups::shared::{
    BasicValue as _, ObjectId, PersonRef, Reference, Revision,
};
use fs_group_directory::groups::{
    GroupDraft, Groups, NestOutcome, NestingMode, Visibility,
};
use fs_group_directory::groups::{AddonKind, FormalOrganization, GroupGovernance, Scope, role_owner};
use fs_group_directory::{FakeGroups, FakePeople, group_ref};
use std::sync::Arc;

fn draft(name: &str, mode: NestingMode) -> GroupDraft {
    GroupDraft {
        name: name.to_owned(),
        kind: None,
        description: None,
        default_mode: mode,
        visibility: Visibility::Everyone,
        sensitive: false,
        mirror: false,
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // The people book belongs to fs-people; the fake brings its own. Alice also installed the project.
    let people = Arc::new(FakePeople::new());
    let alice = people.human("Alice Founder");
    let (bob, carl, dora, admin) = (
        people.human("Bob Second"),
        people.human("Carl Singer"),
        people.human("Dora Stranger"),
        people.human("Ida Admin"),
    );
    let fake = FakeGroups::with_people(people, alice.clone()).with_installer(alice.clone());

    // 1. Alice founds the club; she is its first owner, and only she.
    let club = fake
        .create_owned(draft("Musterstadt Chor", NestingMode::BottomUp))
        .await
        .expect("create");
    println!(
        "owners at the start: {}",
        fake.owners(club.id).await.expect("owners").len()
    );

    // 2. She appoints Bob (several owners are the normal case); Carl joins as a plain member.
    fake.appoint_owner(club.id, &bob).await.expect("appoint");
    fake.add_member(club.id, &carl, None).await.expect("member");
    println!(
        "owners now: {} (role {})",
        fake.owners(club.id).await.expect("owners").len(),
        role_owner().0.canonical()
    );

    // 3. The club is a plain group. Now it is attached the addon "formal organisation" (W3C ORG), by its own owner.
    let form = AddonKind::FormalOrganization(FormalOrganization {
        legal_name: "Musterstadt Chor e.V.".to_owned(),
        legal_form: Some("Verein".to_owned()),
        registration: Some("VR 1234".to_owned()),
    });
    let before = fake
        .effective_members(club.id, None, 100)
        .await
        .expect("members")
        .items
        .len();
    let attached = fake.attach_addon(club.id, form).await.expect("attach");
    let tax = Reference::parse("https://example.org/ns/addons/tax-exempt").expect("an IRI");
    fake.attach_addon(club.id, AddonKind::Other(tax))
        .await
        .expect("second addon");
    let after = fake
        .effective_members(club.id, None, 100)
        .await
        .expect("members")
        .items
        .len();
    assert_eq!(before, after, "an addon changes nothing else");
    println!(
        "addons: {}; members before and after: {before} / {after}",
        fake.addons(club.id).await.expect("addons").len()
    );

    // 4. Self-management: Bob (second owner) changes the addon; Carl (plain member) and Dora (stranger) cannot.
    fake.set_asker(bob.clone());
    let changed = FormalOrganization {
        legal_name: "Musterstadt Chor e.V. (neu)".to_owned(),
        legal_form: Some("Verein".to_owned()),
        registration: Some("VR 1234".to_owned()),
    };
    fake.update_addon(
        attached.id,
        attached.revision,
        AddonKind::FormalOrganization(changed),
    )
    .await
    .expect("Bob may");
    for who in [&carl, &dora] {
        fake.set_asker(who.clone());
        let r = fake
            .detach_addon(attached.id, Revision(attached.revision.0 + 1))
            .await;
        println!("detach by a non-manager: {:?}", r.err());
    }

    // 5. A document owned by the club: the owner is a group reference; it resolves to people.
    fake.set_asker(alice.clone());
    let owners = fake
        .resolve_owner(&group_ref(club.id))
        .await
        .expect("resolve");
    println!("document owner (group) resolves to {} people", owners.len());

    federation_and_admin(&fake, club.id, &admin, dora).await;
}

/// Steps 6 and 7: the edge mode per edge, and the intervention of level (a).
async fn federation_and_admin(
    fake: &FakeGroups,
    club: ObjectId,
    admin: &PersonRef,
    dora: PersonRef,
) {
    // 6. The club nests under a federation; the edge takes the federation's default_mode (TopDown here).
    let fed = fake
        .create_owned(draft("Chorverband", NestingMode::TopDown))
        .await
        .expect("federation");
    match fake.nest_default(club, fed.id).await.expect("nest") {
        NestOutcome::Linked(edge) => {
            println!("edge mode from the federation's default: {:?}", edge.mode);
        }
        NestOutcome::Requested(_) => println!("the other side must agree first"),
    }

    // 7. Level (a): Ida's admin group holds Project; she steps in WITH a reason; everyone who reads the club sees it.
    let admins = fake
        .create_owned(draft("Project admins", NestingMode::Single))
        .await
        .expect("admins");
    fake.add_member(admins.id, admin, None)
        .await
        .expect("admin");
    fake.grant_scope(admins.id, Scope::Project)
        .await
        .expect("scope");
    fake.set_asker(admin.clone());
    fake.open_intervention(
        club,
        Scope::Project,
        "Mitglied fehlt, Chor-Leitung nicht erreichbar",
    )
    .await
    .expect("intervene");
    fake.add_member(club, &dora, None).await.expect("act");
    fake.set_asker(dora);
    for i in fake.interventions(club).await.expect("visible") {
        println!("intervention: {} (acts: {})", i.reason, i.acts.len());
    }
}
