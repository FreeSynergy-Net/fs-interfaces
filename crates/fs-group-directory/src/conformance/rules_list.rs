//! Rules: the list with its filters and paging.
//!
//! Pattern: Template method (a fixed list of rules; each rule seeds its own data through the trait).

use super::cx::Cx;
use crate::groups_v1::{GroupQuery, NestingMode, ParentFilter};
use crate::ids::object_id;

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    listing(cx).await;
}

async fn listing(cx: &mut Cx<'_>) {
    let (Some(p), Some(a), Some(b), Some(c)) = (
        cx.group("pg").await,
        cx.group("pg").await,
        cx.group("pg").await,
        cx.group("pg").await,
    ) else {
        return;
    };
    cx.link(a.id, p.id, NestingMode::Single).await;
    cx.link(b.id, p.id, NestingMode::BottomUp).await;
    let q = GroupQuery {
        text: "zzconf pg".to_owned(),
        parent: ParentFilter::Any,
        member: None,
        include_deleted: false,
    };
    let first = cx.g.list(&q, None, 2).await;
    let Ok(first) = first else {
        cx.ensure(false, "list: answers");
        return;
    };
    cx.ensure(
        first.items.len() == 2 && first.next.is_some(),
        "list: a page of 2 has a next",
    );
    let rest =
        cx.g.list(&q, first.next.as_ref(), 10)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    let mut ids: Vec<_> = first
        .items
        .iter()
        .chain(rest.iter())
        .map(|s| s.id)
        .collect();
    ids.sort();
    ids.dedup();
    cx.ensure(
        ids.len() == 4 && ids.contains(&c.id),
        "list: the pages together hold every group once",
    );
    let names: Vec<String> = first
        .items
        .iter()
        .chain(rest.iter())
        .map(|s| s.name.to_lowercase())
        .collect();
    cx.ensure(
        names.windows(2).all(|w| w[0] <= w[1]),
        "list: sorted by name",
    );
    let kids =
        cx.g.list(
            &GroupQuery {
                parent: ParentFilter::ChildrenOf(p.id),
                ..q.clone()
            },
            None,
            10,
        )
        .await
        .map(|p| p.items)
        .unwrap_or_default();
    cx.ensure(
        kids.len() == 2,
        "list: ChildrenOf lists the direct children (all modes)",
    );
    let roots =
        cx.g.list(
            &GroupQuery {
                parent: ParentFilter::Roots,
                ..q.clone()
            },
            None,
            10,
        )
        .await
        .map(|p| p.items)
        .unwrap_or_default();
    cx.ensure(
        roots.iter().all(|s| s.id != a.id && s.id != b.id) && roots.iter().any(|s| s.id == p.id),
        "list: Roots has no group with a parent",
    );
    cx.ensure(
        cx.g.list(
            &GroupQuery {
                parent: ParentFilter::ChildrenOf(object_id(8_888_890)),
                ..q.clone()
            },
            None,
            10,
        )
        .await
        .is_err(),
        "list: ChildrenOf of an unknown group is an error",
    );
    let me = cx.me.clone();
    cx.member(c.id, &me).await;
    let mine =
        cx.g.list(
            &GroupQuery {
                member: Some(me),
                ..q.clone()
            },
            None,
            10,
        )
        .await
        .map(|p| p.items)
        .unwrap_or_default();
    cx.ensure(
        mine.len() == 1 && mine[0].id == c.id,
        "list: the member filter keeps groups the person is a direct member of",
    );
    let row =
        cx.g.list(&q, None, 10)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    let top = row.iter().find(|s| s.id == p.id);
    cx.ensure(
        top.is_some_and(|s| s.children == 2 && s.parents == 0),
        "list: parents and children are counted",
    );
    cx.ensure(
        row.iter()
            .find(|s| s.id == c.id)
            .is_some_and(|s| s.direct_members == Some(1)),
        "list: direct_members counts where the list may be seen",
    );
}
