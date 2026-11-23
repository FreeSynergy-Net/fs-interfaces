//! Rules: an asking AI does not see admin groups and sensitive groups at all (setup, lists, numbers and every
//! method that takes a group id: `NotFound` before any other error).
//!
//! Pattern: Template method (the same suite runs for an admin group and for a sensitive group; the ids of the
//! objects of the hidden group are collected as a human, then used as the AI).

use super::cx::{Cx, err};
use super::fixture::Kind;
use crate::groups_v1::shared::{ObjectId, Revision};
use crate::groups_v1::{Error, GroupQuery, NestOutcome, NestingMode, ParentFilter, Right};
use crate::ids::group_ref;

/// A hidden group with a plain neighbour and the ids of the things that hang on it.
pub(crate) struct Hid {
    pub group: ObjectId,
    pub neighbour: ObjectId,
    pub low: ObjectId,
    pub member: (ObjectId, Revision),
    pub invitation: (ObjectId, Revision),
    pub grant: (ObjectId, Revision),
    pub edge: (ObjectId, Revision),
    pub request: (ObjectId, Revision),
    pub agent: crate::groups_v1::shared::PersonRef,
    pub human: crate::groups_v1::shared::PersonRef,
}

pub(crate) fn is_not_found<T>(r: &crate::groups_v1::Result<T>) -> bool {
    matches!(err(r), Some(Error::NotFound(_)))
}

/// Make the hidden group (`admin` = with an admin role, else sensitive) and everything around it, as the human.
async fn setup(cx: &mut Cx<'_>, admin: bool) -> Option<Hid> {
    let group = if admin {
        let g = cx.group("hid admin").await?;
        let (_, role) = cx.fx.roles().await?;
        let boss = cx.person(Kind::Human).await?;
        cx.g.add_member(g.id, &boss, Some(&role)).await.ok()?;
        g
    } else {
        cx.sensitive_group("hid sens").await?
    };
    let member = cx.person(Kind::Human).await?;
    let human = cx.person(Kind::Human).await?;
    let agent = cx.person(Kind::Agent).await?;
    let m = cx.g.add_member(group.id, &member, None).await.ok()?;
    let inv = cx.g.invite(group.id, &human, None, None).await.ok()?;
    let grant = cx.g.grants(group.id).await.ok()?.into_iter().next()?;
    let neighbour = cx.group("hid plain").await?;
    let low = cx.group("hid low").await?;
    let mode = if admin {
        NestingMode::BottomUp
    } else {
        NestingMode::Single
    };
    let edge = cx.link(group.id, neighbour.id, mode).await?;
    cx.link(low.id, group.id, NestingMode::Single).await?;
    cx.g.grant(neighbour.id, &group_ref(group.id), Right::Read)
        .await
        .ok()?;
    cx.g.grant(neighbour.id, &agent, Right::Read).await.ok()?;
    let h2 = cx.person(Kind::Human).await?;
    if !cx.act(&h2).await {
        return None;
    }
    let other = cx.group("hid other").await;
    let out = match other {
        Some(o) => {
            cx.g.request_nesting(o.id, group.id, NestingMode::Single)
                .await
        }
        None => return None,
    };
    cx.act_me().await;
    let Ok(NestOutcome::Requested(req)) = out else {
        return None;
    };
    Some(Hid {
        group: group.id,
        neighbour: neighbour.id,
        low: low.id,
        member: (m.id, m.revision),
        invitation: (inv.id, inv.revision),
        grant: (grant.id, grant.revision),
        edge: (edge.id, edge.revision),
        request: (req.id, req.revision),
        agent,
        human,
    })
}

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    for admin in [true, false] {
        let name = if admin {
            "admin group"
        } else {
            "sensitive group"
        };
        let Some(h) = setup(cx, admin).await else {
            cx.skip(&format!("hidden: setup of the {name} (the fixture cannot)"));
            continue;
        };
        // What the human sees, to compare with.
        let seen =
            cx.g.list(&query(), None, 500)
                .await
                .map(|p| p.items)
                .unwrap_or_default();
        let human_children = seen
            .iter()
            .find(|s| s.id == h.neighbour)
            .map_or(0, |s| s.children);
        let human_roots =
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
            human_children == 1 && !human_roots.iter().any(|s| s.id == h.low),
            "setup: the human sees the neighbour's child and the low group has a parent",
        );
        let before = cx.g.changes(None, 5).await.ok().and_then(|p| p.next);
        cx.g.add_member(h.group, &h.human, None).await.ok();
        if let Some(x) = cx.person(Kind::Human).await {
            cx.g.add_member(h.neighbour, &x, None).await.ok();
        }
        if !cx.act(&h.agent).await {
            continue;
        }
        super::rules_hidden_ids::rules(cx, &h, name).await;
        super::rules_hidden_sub::rules(cx, &h, name, before).await;
        cx.act_me().await;
    }
}

pub(crate) fn query() -> GroupQuery {
    GroupQuery {
        text: "Zzconf hid".to_owned(),
        parent: ParentFilter::Any,
        member: None,
        include_deleted: true,
    }
}
