//! Rules: the hidden group, part one: lists, numbers, filters and every method that takes a group id.
//!
//! Pattern: Template method (the same suite runs for an admin group and for a sensitive group).

use super::cx::Cx;
use super::rules_hidden::{Hid, is_not_found, query};
use crate::groups_v1::shared::{PersonArt, Revision, Utc};
use crate::groups_v1::{
    GroupPatch, GroupQuery, NestingMode, NestingSide, ParentFilter, RequestSide, Right,
};
use crate::ids::{group_ref, object_id, person};

pub(crate) async fn rules(cx: &mut Cx<'_>, h: &Hid, name: &str) {
    by_id(cx, h, name).await;
    lists(cx, h, name).await;
}

/// Lists, numbers and filters: only what the AI may see counts.
async fn lists(cx: &mut Cx<'_>, h: &Hid, name: &str) {
    let all =
        cx.g.list(&query(), None, 500)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    cx.ensure(
        all.iter().all(|s| s.id != h.group) && all.iter().any(|s| s.id == h.neighbour),
        &format!("hidden {name}: list leaves it out and keeps the rest"),
    );
    cx.ensure(
        all.iter()
            .find(|s| s.id == h.neighbour)
            .is_some_and(|s| s.children == 0),
        &format!("hidden {name}: children counts only visible groups"),
    );
    let roots =
        cx.g.list(
            &GroupQuery {
                parent: ParentFilter::Roots,
                ..query()
            },
            None,
            500,
        )
        .await
        .map(|p| p.items)
        .unwrap_or_default();
    cx.ensure(
        roots.iter().any(|s| s.id == h.low),
        &format!("hidden {name}: Roots means no VISIBLE parent"),
    );
    cx.ensure(
        all.iter()
            .find(|s| s.id == h.low)
            .is_some_and(|s| s.parents == 0),
        &format!("hidden {name}: parents counts only visible groups"),
    );
    let kids =
        cx.g.list(
            &GroupQuery {
                parent: ParentFilter::ChildrenOf(h.group),
                ..query()
            },
            None,
            50,
        )
        .await;
    let unknown =
        cx.g.list(
            &GroupQuery {
                parent: ParentFilter::ChildrenOf(object_id(8_888_891)),
                ..query()
            },
            None,
            50,
        )
        .await;
    cx.ensure(
        is_not_found(&kids) && is_not_found(&unknown),
        &format!("hidden {name}: ChildrenOf a hidden group is NotFound, as for an unknown one"),
    );
    cx.ensure(
        cx.g.nestings(h.neighbour, NestingSide::AsParent)
            .await
            .is_ok_and(|v| v.iter().all(|e| e.id != h.edge.0)),
        &format!("hidden {name}: nestings leaves the edge out"),
    );
    cx.ensure(
        cx.g.ancestors(h.low).await.is_ok_and(|v| v.is_empty()),
        &format!("hidden {name}: ancestors does not pass through a hidden group"),
    );
    cx.ensure(
        cx.g.descendants(h.neighbour)
            .await
            .is_ok_and(|v| !v.contains(&h.group) && !v.contains(&h.low)),
        &format!("hidden {name}: descendants does not pass through a hidden group"),
    );
    cx.ensure(
        cx.g.grants(h.neighbour)
            .await
            .is_ok_and(|v| v.iter().all(|g| g.holder.art != PersonArt::Group)),
        &format!("hidden {name}: a grant held by a hidden group is left out"),
    );
    cx.ensure(
        cx.g.history(h.neighbour, None, 99).await.is_ok_and(|p| {
            p.items
                .iter()
                .all(|e| !format!("{:?}", e.kind).contains(&format!("{:?}", h.group)))
        }),
        &format!("hidden {name}: the history of a neighbour does not name it"),
    );
    let eff =
        cx.g.effective_members(h.neighbour, None, 99)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    cx.ensure(
        eff.iter().all(|m| {
            m.inherited
                .iter()
                .all(|i| i.from != Some(h.group) && (i.from.is_some() || i.via.is_empty()))
        }),
        &format!("hidden {name}: the origin of a hidden group reads from = None, via empty"),
    );
    cx.ensure(
        cx.g.groups_of(&h.human)
            .await
            .is_ok_and(|v| v.iter().all(|e| e.group != h.group)),
        &format!("hidden {name}: groups_of leaves it out"),
    );
    cx.ensure(
        is_not_found(&cx.g.may_do(h.group, &h.human, Right::Read).await),
        &format!("hidden {name}: may_do about another person is NotFound too"),
    );
}

/// Every method that takes a group id, with a hidden group and with an input that would fail first otherwise.
async fn by_id(cx: &mut Cx<'_>, h: &Hid, name: &str) {
    let g = h.group;
    let stale = Revision(999);
    let past = Utc::from_millis(1);
    let mut org = group_ref(g);
    org.art = PersonArt::Organization;
    let checks: [(&str, bool); 22] = [
        ("get", matches!(cx.g.get(g).await, Ok(None))),
        (
            "get_by_reference",
            matches!(
                cx.g.get_by_reference(&crate::ids::reference_of(g)).await,
                Ok(None)
            ),
        ),
        (
            "update",
            is_not_found(
                &cx.g
                    .update(
                        g,
                        stale,
                        GroupPatch {
                            name: Some(String::new()),
                            ..GroupPatch::default()
                        },
                    )
                    .await,
            ),
        ),
        ("delete", is_not_found(&cx.g.delete(g, stale).await)),
        ("restore", is_not_found(&cx.g.restore(g).await)),
        (
            "request_nesting as child",
            is_not_found(
                &cx.g
                    .request_nesting(g, h.neighbour, NestingMode::Single)
                    .await,
            ),
        ),
        (
            "request_nesting as parent",
            is_not_found(
                &cx.g
                    .request_nesting(h.neighbour, g, NestingMode::BottomUp)
                    .await,
            ),
        ),
        (
            "request_nesting to itself",
            is_not_found(&cx.g.request_nesting(g, g, NestingMode::BottomUp).await),
        ),
        (
            "nesting_effect as child",
            is_not_found(
                &cx.g
                    .nesting_effect(g, h.neighbour, NestingMode::BottomUp)
                    .await,
            ),
        ),
        (
            "nesting_effect as parent",
            is_not_found(
                &cx.g
                    .nesting_effect(h.neighbour, g, NestingMode::BottomUp)
                    .await,
            ),
        ),
        (
            "nestings as child",
            is_not_found(&cx.g.nestings(g, NestingSide::AsChild).await),
        ),
        (
            "nestings as parent",
            is_not_found(&cx.g.nestings(g, NestingSide::AsParent).await),
        ),
        (
            "nesting_requests incoming",
            is_not_found(&cx.g.nesting_requests(g, RequestSide::Incoming).await),
        ),
        (
            "nesting_requests outgoing",
            is_not_found(&cx.g.nesting_requests(g, RequestSide::Outgoing).await),
        ),
        ("ancestors", is_not_found(&cx.g.ancestors(g).await)),
        ("descendants", is_not_found(&cx.g.descendants(g).await)),
        ("members", is_not_found(&cx.g.members(g, None, 5).await)),
        (
            "effective_members",
            is_not_found(&cx.g.effective_members(g, None, 5).await),
        ),
        (
            "add_member with a group as person",
            is_not_found(&cx.g.add_member(g, &group_ref(g), None).await),
        ),
        ("invitations", is_not_found(&cx.g.invitations(g).await)),
        (
            "invite with an expiry in the past",
            is_not_found(&cx.g.invite(g, &h.human, None, Some(past)).await),
        ),
        ("grants", is_not_found(&cx.g.grants(g).await)),
    ];
    for (method, ok) in checks {
        cx.ensure(
            ok,
            &format!("hidden {name}: {method} is NotFound (checked before any other error)"),
        );
    }
    cx.ensure(
        is_not_found(&cx.g.grant(g, &org, Right::Read).await),
        &format!("hidden {name}: grant is NotFound before WrongArt"),
    );
    cx.ensure(
        is_not_found(&cx.g.may_do(g, &person(8_000_002), Right::Read).await),
        &format!("hidden {name}: may_do is NotFound before an unknown person"),
    );
    cx.ensure(
        is_not_found(&cx.g.my_rights(g).await),
        &format!("hidden {name}: my_rights is NotFound"),
    );
    cx.ensure(
        is_not_found(&cx.g.history(g, None, 5).await),
        &format!("hidden {name}: history is NotFound"),
    );
}
