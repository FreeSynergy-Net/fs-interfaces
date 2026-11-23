//! The nesting logic on awkward data: a cycle named the same way whatever the order of the edges, and a depth check
//! that ends on old data with a cycle (the fake used to overflow its stack there).

use fs_group_directory::FakeGroups;
use fs_group_directory::groups::shared::ObjectId;
use fs_group_directory::groups::{
    Error, GroupDraft, Groups, NestOutcome, NestingMode, Visibility,
};

fn draft(name: &str) -> GroupDraft {
    GroupDraft {
        name: name.to_owned(),
        kind: None,
        description: None,
        default_mode: NestingMode::BottomUp,
        visibility: Visibility::Everyone,
        sensitive: false,
        mirror: false,
    }
}

async fn groups(fake: &FakeGroups, n: usize) -> Vec<ObjectId> {
    let mut out = Vec::new();
    for i in 0..n {
        out.push(
            fake.create(draft(&format!("G{i}")))
                .await
                .expect("create")
                .id,
        );
    }
    out
}

/// P is below X and Y, both are below C: closing C below P makes two cycles of the same length (through X or Y).
/// The path named is the smallest in id order, whatever edge came first.
async fn cycle_path(x_first: bool) -> (Vec<ObjectId>, Vec<ObjectId>) {
    let fake = FakeGroups::new();
    let ids = groups(&fake, 4).await;
    let (c, p, x, y) = (ids[0], ids[1], ids[2], ids[3]);
    let mut pairs = [(p, x), (p, y), (x, c), (y, c)];
    if !x_first {
        pairs.reverse();
    }
    for (child, parent) in pairs {
        let made = fake
            .request_nesting(child, parent, NestingMode::Single)
            .await;
        assert!(matches!(made, Ok(NestOutcome::Linked(_))), "{made:?}");
    }
    let Err(Error::Cycle { path }) = fake.request_nesting(c, p, NestingMode::Single).await else {
        panic!("a cycle was expected");
    };
    (path, vec![c, p, x.min(y), c])
}

#[tokio::test]
async fn of_several_equally_short_cycles_the_smallest_path_is_named_whatever_the_edge_order() {
    let (first, want) = cycle_path(true).await;
    let (second, _) = cycle_path(false).await;
    assert_eq!(first, want, "smallest in id order");
    assert_eq!(first, second, "the order of the edges does not matter");
}

#[tokio::test]
async fn the_depth_check_ends_on_old_data_with_a_cycle() {
    let fake = FakeGroups::new();
    let ids = groups(&fake, 4).await;
    fake.insert_edge_raw(ids[0], ids[1], NestingMode::BottomUp);
    fake.insert_edge_raw(ids[1], ids[2], NestingMode::BottomUp);
    fake.insert_edge_raw(ids[2], ids[0], NestingMode::BottomUp);
    let made = fake
        .request_nesting(ids[3], ids[0], NestingMode::BottomUp)
        .await;
    assert!(
        matches!(made, Ok(NestOutcome::Linked(_))),
        "ends, no stack overflow: {made:?}"
    );
    assert!(
        fake.nesting_effect(ids[3], ids[2], NestingMode::Single)
            .await
            .is_ok()
    );
}
