//! Rules: the trash (`delete`, `restore`) and the list with its filters and paging.
//!
//! Pattern: Template method (a fixed list of rules; each rule seeds its own data through the trait).

use super::cx::{Cx, err};
use super::fixture::Kind;
use crate::groups::shared::Revision;
use crate::groups::{
    Error, GroupQuery, HistoryKind, NestingMode, NestingSide, ParentFilter, Right,
};
use crate::ids::object_id;

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    trash(cx).await;
    trash_edges(cx).await;
    super::rules_list::rules(cx).await;
}

async fn trash(cx: &mut Cx<'_>) {
    let Some(g) = cx.group("trash").await else {
        return;
    };
    let Some(p) = cx.person(Kind::Human).await else {
        return;
    };
    cx.member(g.id, &p).await;
    cx.ensure(
        matches!(
            err(&cx.g.delete(g.id, Revision(g.revision.0 + 99)).await),
            Some(Error::Changed { .. })
        ),
        "delete: an old revision is Changed",
    );
    let Ok(gone) = cx.g.delete(g.id, g.revision).await else {
        cx.ensure(false, "delete: the owner may delete");
        return;
    };
    cx.ensure(
        matches!(cx.g.get(g.id).await, Ok(None)),
        "delete: get of a deleted group is None",
    );
    let all = GroupQuery {
        text: "Zzconf trash".to_owned(),
        parent: ParentFilter::Any,
        member: None,
        include_deleted: true,
    };
    let page =
        cx.g.list(&all, None, 50)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    cx.ensure(
        page.iter()
            .any(|s| s.id == g.id && s.restorable_until == Some(gone.restorable_until)),
        "delete: include_deleted shows it with restorable_until",
    );
    let page =
        cx.g.list(
            &GroupQuery {
                include_deleted: false,
                ..all.clone()
            },
            None,
            50,
        )
        .await
        .map(|p| p.items)
        .unwrap_or_default();
    cx.ensure(
        page.iter().all(|s| s.id != g.id),
        "delete: the default list leaves it out",
    );
    cx.ensure(
        matches!(
            err(&cx.g.delete(g.id, g.revision).await),
            Some(Error::NotFound(_))
        ),
        "delete: twice is NotFound",
    );
    cx.ensure(
        matches!(
            err(&cx.g.members(g.id, None, 5).await),
            Some(Error::NotFound(_))
        ),
        "delete: members of a deleted group is NotFound",
    );
    let back = cx.g.restore(g.id).await;
    cx.ensure(back.is_ok(), "restore: brings the group back");
    cx.ensure(
        cx.g.members(g.id, None, 10)
            .await
            .is_ok_and(|m| m.items.len() == 1),
        "restore: the memberships come back",
    );
    cx.ensure(
        cx.g.my_rights(g.id)
            .await
            .is_ok_and(|r| r.contains(&Right::Manage)),
        "restore: the rights come back",
    );
    cx.ensure(
        matches!(
            err(&cx.g.restore(g.id).await),
            Some(Error::NotRestorable(_))
        ),
        "restore: a living group is NotRestorable",
    );
    cx.ensure(
        matches!(
            err(&cx.g.restore(object_id(8_888_889)).await),
            Some(Error::NotRestorable(_))
        ),
        "restore: an unknown id is NotRestorable",
    );
    let rev = cx.rev(g.id).await;
    if cx.g.delete(g.id, rev).await.is_ok() {
        if cx.fx.touch_deleted(g.id).await {
            cx.ensure(
                matches!(err(&cx.g.restore(g.id).await), Some(Error::Changed { .. })),
                "restore: a group changed since the deletion is Changed",
            );
        } else {
            cx.skip("restore: Changed (the fixture cannot touch a deleted group)");
        }
        if cx.fx.advance_clock(31 * 24 * 3600 * 1000).await {
            cx.ensure(
                matches!(
                    err(&cx.g.restore(g.id).await),
                    Some(Error::NotRestorable(_))
                ),
                "restore: after the deadline it is NotRestorable",
            );
        } else {
            cx.skip("restore: the deadline (the fixture cannot move the clock)");
        }
    }
}

/// Edges taken by `delete`, brought back by `restore` unless they would close a cycle meanwhile.
async fn trash_edges(cx: &mut Cx<'_>) {
    let (Some(a), Some(b), Some(c)) = (
        cx.group("te a").await,
        cx.group("te b").await,
        cx.group("te c").await,
    ) else {
        return;
    };
    cx.link(a.id, b.id, NestingMode::BottomUp).await;
    cx.link(b.id, c.id, NestingMode::BottomUp).await;
    cx.ensure(
        cx.g.delete(b.id, b.revision).await.is_ok(),
        "delete: a group in the middle",
    );
    cx.ensure(
        cx.g.nestings(a.id, NestingSide::AsChild)
            .await
            .is_ok_and(|e| e.is_empty()),
        "delete: the children lose this parent",
    );
    cx.link(c.id, a.id, NestingMode::BottomUp).await;
    cx.ensure(
        cx.g.restore(b.id).await.is_ok(),
        "restore: a group whose edges would close a cycle",
    );
    let up =
        cx.g.nestings(a.id, NestingSide::AsChild)
            .await
            .map_or(0, |e| e.len());
    let down =
        cx.g.nestings(b.id, NestingSide::AsChild)
            .await
            .map_or(0, |e| e.len());
    cx.ensure(
        up + down == 1,
        "restore: the edge that would close a cycle stays away, the other comes back",
    );
    let h =
        cx.g.history(b.id, None, 50)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    cx.ensure(
        h.iter()
            .any(|e| matches!(e.kind, HistoryKind::NestingRemoved { .. })),
        "restore: the dropped edge is a NestingRemoved in the history",
    );
}
