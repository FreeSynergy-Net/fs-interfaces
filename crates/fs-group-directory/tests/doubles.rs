//! `FailingGroups` and `SlowGroups`: the error, partly broken, disconnected and loading states of the frontends.

use fs_group_directory::groups::{
    Error, GroupDraft, GroupMembers, GroupQuery, Groups, Mirroring, NestingMode,
    ParentFilter, Unreachable, Visibility,
};
use fs_group_directory::{
    FailingGroups, Op, SlowGroups, forbidden, group_ref, sample, unavailable,
};
use std::sync::Arc;

fn draft() -> GroupDraft {
    GroupDraft {
        name: "New".to_owned(),
        kind: None,
        description: None,
        default_mode: NestingMode::BottomUp,
        visibility: Visibility::Everyone,
        sensitive: false,
        mirror: false,
    }
}

fn all() -> GroupQuery {
    GroupQuery {
        text: String::new(),
        parent: ParentFilter::Any,
        member: None,
        include_deleted: false,
    }
}

#[tokio::test]
async fn a_failing_provider_answers_the_chosen_error_per_method_and_heals() {
    let s = sample();
    let events = s.id("Events team").expect("group");
    let f = FailingGroups::new(Arc::new(s.groups))
        .failing(Op::List, unavailable())
        .failing(Op::Create, forbidden(events));
    assert_eq!(
        f.list(&all(), None, 5).await.err(),
        Some(Error::Unavailable(Unreachable::Server))
    );
    assert!(matches!(
        f.create(draft()).await,
        Err(Error::Forbidden { .. })
    ));
    assert!(f.get(events).await.is_ok(), "the other methods work");
    f.heal();
    assert!(f.list(&all(), None, 5).await.is_ok());
}

#[tokio::test]
async fn a_failing_provider_can_break_one_group_and_leave_the_others() {
    let s = sample();
    let (events, outreach) = (
        s.id("Events team").expect("group"),
        s.id("Outreach team").expect("group"),
    );
    let f = FailingGroups::new(Arc::new(s.groups)).failing_group(events, unavailable());
    assert!(f.get(events).await.is_err());
    assert!(f.members(events, None, 5).await.is_err());
    assert!(
        f.describe(&group_ref(events)).await.is_err(),
        "the narrow view names the group too"
    );
    assert!(
        f.request_nesting(outreach, events, NestingMode::Single)
            .await
            .is_err(),
        "a call that names the broken group among two"
    );
    assert!(f.get(outreach).await.is_ok());
    assert!(f.members(outreach, None, 5).await.is_ok());
    f.heal();
    assert!(f.get(events).await.is_ok());
}

#[tokio::test]
async fn a_failing_provider_can_be_gone_for_every_method_of_all_three_traits() {
    let s = sample();
    let g = s.id("Events team").expect("group");
    let alice = s.person("Alice Chair").expect("person");
    let f = FailingGroups::new(Arc::new(s.groups)).failing_all(unavailable());
    assert!(f.list(&all(), None, 5).await.is_err());
    assert!(f.my_rights(g).await.is_err());
    assert!(f.memberships_of(&alice).await.is_err());
    assert!(f.mirror_status().await.is_err());
    assert!(
        f.mirror_plan(&s.mirror).await.is_err(),
        "wrapped as MirrorError::Groups"
    );
    f.heal();
    assert!(f.mirror_status().await.is_ok());
}

#[tokio::test]
async fn a_slow_provider_waits_on_the_fakes_clock_and_nobody_sleeps() {
    let s = sample();
    let events = s.id("Events team").expect("group");
    let fake = Arc::new(s.groups);
    let before = fake.now().millis();
    let slow = SlowGroups::of_fake(Arc::clone(&fake), 3_000).taking(Op::List, 10_000);
    let started = std::time::Instant::now();
    slow.list(&all(), None, 5).await.expect("list");
    slow.get(events).await.expect("get");
    assert_eq!(slow.waited_ms(), 13_000);
    assert!(fake.now().millis() >= before + 13_000, "the clock moved");
    assert!(started.elapsed().as_secs() < 2, "no real sleep");
}
