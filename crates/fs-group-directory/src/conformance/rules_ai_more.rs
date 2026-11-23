//! Rules: what an AI account may be given, marks that change after the fact, `restore` and the AI rule.
//!
//! Pattern: Template method (a fixed list of rules; each rule seeds its own data through the trait).

use super::cx::{Cx, err};
use super::fixture::Kind;
use crate::groups_v1::{
    Error, GroupPatch, HistoryKind, InForce, NestingMode, NestingSide, Reason, Right,
};
use crate::ids::group_ref;

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    ai_acts(cx).await;
    remarked(cx).await;
    restore_rules(cx).await;
}

async fn ai_acts(cx: &mut Cx<'_>) {
    let (Some(a), Some(u), Some(g)) = (
        cx.person(Kind::Agent).await,
        cx.person(Kind::Unknown).await,
        cx.group("acts").await,
    ) else {
        return;
    };
    cx.ensure(
        matches!(
            err(&cx.g.grant(g.id, &a, Right::Manage).await),
            Some(Error::AgentNotAdmin)
        ),
        "AI rule: Manage to an AI account is AgentNotAdmin",
    );
    cx.ensure(
        matches!(
            err(&cx.g.grant(g.id, &u, Right::Manage).await),
            Some(Error::AiStatusUnknown)
        ),
        "AI rule: Manage to an unknown person is AiStatusUnknown",
    );
    cx.ensure(
        cx.g.grant(g.id, &a, Right::Read).await.is_ok(),
        "AI rule: an AI account may get Read, expressly and singly",
    );
    cx.ensure(
        cx.g.may_do(g.id, &a, Right::Manage)
            .await
            .is_ok_and(|d| !d.allowed && d.reason == Reason::Agent),
        "may_do: an AI is never Manage (Agent)",
    );
    cx.ensure(
        cx.g.may_do(g.id, &a, Right::Read)
            .await
            .is_ok_and(|d| d.allowed && matches!(d.reason, Reason::Holder { .. })),
        "may_do: an AI holds what it was given",
    );
    cx.ensure(
        cx.g.may_do(g.id, &u, Right::Manage)
            .await
            .is_ok_and(|d| !d.allowed && d.reason == Reason::AiStatusUnknown),
        "may_do: an unknown person is never Manage",
    );
    let Some(ai_group) = cx.ai_group("acts ai").await else {
        return;
    };
    cx.ensure(
        matches!(
            err(&cx
                .g
                .grant(g.id, &group_ref(ai_group.id), Right::Manage)
                .await),
            Some(Error::AgentNotAdmin)
        ),
        "AI rule: Manage to an AI group is AgentNotAdmin",
    );
    if cx.act(&a).await {
        let d = cx.draft("by ai");
        cx.ensure(
            matches!(err(&cx.g.create(d).await), Some(Error::AgentNotAdmin)),
            "create: an asking AI gets AgentNotAdmin and nothing is written",
        );
        cx.ensure(
            matches!(
                err(&cx
                    .g
                    .update(
                        g.id,
                        g.revision,
                        GroupPatch {
                            ai: Some(true),
                            ..GroupPatch::default()
                        }
                    )
                    .await),
                Some(Error::Forbidden {
                    right: Right::Manage,
                    ..
                })
            ),
            "update: an asking AI has no Manage, Forbidden",
        );
        cx.act_me().await;
    }
    let Some((_, admin)) = cx.fx.roles().await else {
        cx.skip("AI rule: admin roles (the fixture has no roles)");
        return;
    };
    let (Some(m1), Some(m2)) = (
        cx.member(ai_group.id, &a).await,
        cx.person(Kind::Unknown).await,
    ) else {
        return;
    };
    let r = cx.g.set_role(m1.id, m1.revision, Some(&admin)).await;
    cx.ensure(
        matches!(err(&r), Some(Error::AgentNotAdmin)),
        "AI rule: an admin role for an AI is AgentNotAdmin",
    );
    cx.ensure(
        matches!(
            err(&cx.g.add_member(g.id, &m2, Some(&admin)).await),
            Some(Error::AiStatusUnknown)
        ),
        "AI rule: an admin role for an unknown person is AiStatusUnknown",
    );
}

/// A mark that changes after the fact: the membership stays and does not count (`InForce::KindMismatch`).
async fn remarked(cx: &mut Cx<'_>) {
    let (Some(g), Some(ai), Some(p), Some(a)) = (
        cx.group("rm").await,
        cx.ai_group("rm ai").await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Agent).await,
    ) else {
        return;
    };
    let (Some(mp), Some(ma)) = (cx.member(g.id, &p).await, cx.member(ai.id, &a).await) else {
        return;
    };
    if !cx.fx.mark(&p, Kind::Agent).await || !cx.fx.mark(&a, Kind::Human).await {
        cx.skip("InForce: KindMismatch (the fixture cannot change the mark)");
        return;
    }
    let held =
        cx.g.members(g.id, None, 5)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    cx.ensure(
        held.len() == 1 && held[0].id == mp.id && held[0].in_force == InForce::KindMismatch,
        "InForce: an AI in a human group stays and is KindMismatch",
    );
    cx.ensure(
        !cx.is_effective(g.id, &p).await,
        "InForce: it is not an effective member",
    );
    cx.ensure(
        cx.g.groups_of(&p).await.is_ok_and(|v| v.is_empty()),
        "InForce: groups_of leaves it out",
    );
    let held =
        cx.g.members(ai.id, None, 5)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    cx.ensure(
        held.len() == 1 && held[0].id == ma.id && held[0].in_force == InForce::KindMismatch,
        "InForce: a human in an AI group is KindMismatch too",
    );
    cx.ensure(
        !cx.is_effective(ai.id, &a).await,
        "InForce: it is not an effective member of the AI group",
    );
    cx.fx.mark(&p, Kind::Human).await;
    cx.ensure(
        cx.is_effective(g.id, &p).await,
        "InForce: marked as a human again it counts again",
    );
    cx.ensure(
        cx.g.grant(g.id, &p, Right::Manage).await.is_ok(),
        "grant: Manage to a human",
    );
    cx.fx.mark(&p, Kind::Agent).await;
    let gs = cx.g.grants(g.id).await.unwrap_or_default();
    cx.ensure(
        gs.iter()
            .any(|x| x.holder == p && x.in_force == InForce::KindMismatch),
        "InForce: a Manage grant to a person marked as AI is KindMismatch",
    );
    cx.ensure(
        cx.g.may_do(g.id, &p, Right::Manage)
            .await
            .is_ok_and(|d| !d.allowed),
        "may_do: and it does not count",
    );
}

/// `restore` checks the AI rule again: the edge stays away, the membership comes back and does not count.
async fn restore_rules(cx: &mut Cx<'_>) {
    let (Some(parent), Some(h), Some(p)) = (
        cx.group("rr parent").await,
        cx.group("rr h").await,
        cx.person(Kind::Human).await,
    ) else {
        return;
    };
    cx.link(h.id, parent.id, NestingMode::BottomUp).await;
    cx.member(h.id, &p).await;
    let rev = cx.rev(h.id).await;
    cx.ensure(
        cx.g.delete(h.id, rev).await.is_ok(),
        "restore: setup, a deleted child",
    );
    let rev = cx.rev(parent.id).await;
    cx.ensure(
        cx.g.update(
            parent.id,
            rev,
            GroupPatch {
                ai: Some(true),
                ..GroupPatch::default()
            },
        )
        .await
        .is_ok(),
        "update(ai): the parent lost its child, so it may become an AI group",
    );
    let fx_ok = cx.fx.mark(&p, Kind::Agent).await;
    cx.ensure(
        cx.g.restore(h.id).await.is_ok(),
        "restore: the group comes back",
    );
    cx.ensure(
        cx.g.nestings(h.id, NestingSide::AsChild)
            .await
            .is_ok_and(|e| e.is_empty()),
        "restore: an edge the AI rule now forbids stays away",
    );
    let log =
        cx.g.history(h.id, None, 50)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    cx.ensure(
        log.iter()
            .any(|e| matches!(e.kind, HistoryKind::NestingRemoved { .. })),
        "restore: the dropped edge is a NestingRemoved in the history",
    );
    if fx_ok {
        let m =
            cx.g.members(h.id, None, 5)
                .await
                .map(|p| p.items)
                .unwrap_or_default();
        cx.ensure(
            m.len() == 1 && m[0].in_force == InForce::KindMismatch,
            "restore: a membership that now breaks the rule comes back and does not count",
        );
    }
}
