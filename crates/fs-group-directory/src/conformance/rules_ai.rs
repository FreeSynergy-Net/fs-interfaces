//! Rules: the AI rule (an AI account only in AI groups, AI groups only as Single leaves), `InForce`, admin roles.
//!
//! Pattern: Template method (a fixed list of rules; the people are made through the fixture).

use super::cx::{Cx, err};
use super::fixture::Kind;
use crate::groups_v1::{Error, GroupPatch, NestOutcome, NestingMode, NestingSide};

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    kinds(cx).await;
    ai_edges(cx).await;
    ai_switch(cx).await;
    super::rules_ai_more::rules(cx).await;
}

pub(crate) fn ai_nesting(e: Option<&Error>) -> bool {
    matches!(e, Some(Error::AiNesting { .. }))
}

async fn kinds(cx: &mut Cx<'_>) {
    let (Some(human), Some(group)) = (cx.group("ai h").await, cx.ai_group("ai a").await) else {
        return;
    };
    let (Some(h), Some(a), Some(u)) = (
        cx.person(Kind::Human).await,
        cx.person(Kind::Agent).await,
        cx.person(Kind::Unknown).await,
    ) else {
        return;
    };
    cx.ensure(
        ai_nesting(err(&cx.g.add_member(human.id, &a, None).await)),
        "AI rule: an AI account in a human group is AiNesting",
    );
    cx.ensure(
        ai_nesting(err(&cx.g.add_member(group.id, &h, None).await)),
        "AI rule: a human in an AI group is AiNesting",
    );
    cx.ensure(
        matches!(
            err(&cx.g.add_member(group.id, &u, None).await),
            Some(Error::AiStatusUnknown)
        ),
        "AI rule: an unknown person in an AI group is AiStatusUnknown",
    );
    cx.ensure(
        cx.g.add_member(human.id, &u, None).await.is_ok(),
        "AI rule: an unknown person is allowed in a human group",
    );
    cx.ensure(
        cx.g.add_member(group.id, &a, None).await.is_ok(),
        "AI rule: an AI account in an AI group",
    );
    cx.ensure(
        ai_nesting(err(&cx.g.invite(human.id, &a, None, None).await)),
        "AI rule: invite follows the same rule (AI in a human group)",
    );
    cx.ensure(
        ai_nesting(err(&cx.g.invite(group.id, &h, None, None).await)),
        "AI rule: invite follows the same rule (human in an AI group)",
    );
    cx.ensure(
        matches!(
            err(&cx.g.invite(group.id, &u, None, None).await),
            Some(Error::AiStatusUnknown)
        ),
        "AI rule: invite of an unknown person into an AI group",
    );
    cx.ensure(
        ai_nesting(err(&cx.g.add_member(group.id, &h, None).await))
            && cx
                .g
                .members(group.id, None, 9)
                .await
                .is_ok_and(|m| m.items.len() == 1),
        "AI rule: a refused write writes nothing",
    );
}

async fn ai_edges(cx: &mut Cx<'_>) {
    let (Some(ai), Some(plain)) = (cx.ai_group("ae ai").await, cx.group("ae plain").await) else {
        return;
    };
    for mode in [NestingMode::BottomUp, NestingMode::TopDown] {
        cx.ensure(
            ai_nesting(err(&cx.g.request_nesting(ai.id, plain.id, mode).await)),
            "AI rule: an AI group as a child is Single only",
        );
        cx.ensure(
            ai_nesting(err(&cx.g.request_nesting(plain.id, ai.id, mode).await)),
            "AI rule: an AI group is never a parent",
        );
    }
    cx.ensure(
        ai_nesting(err(&cx
            .g
            .request_nesting(plain.id, ai.id, NestingMode::Single)
            .await)),
        "AI rule: an AI group has no children, not even Single",
    );
    cx.ensure(
        matches!(
            cx.g.request_nesting(ai.id, plain.id, NestingMode::Single)
                .await,
            Ok(NestOutcome::Linked(_))
        ),
        "AI rule: an AI group may be a Single leaf",
    );
    let edge =
        cx.g.nestings(ai.id, NestingSide::AsChild)
            .await
            .ok()
            .and_then(|v| v.into_iter().next());
    if let Some(e) = edge {
        let r =
            cx.g.change_nesting_mode(e.id, e.revision, NestingMode::BottomUp)
                .await;
        cx.ensure(
            ai_nesting(err(&r)),
            "AI rule: a mode change to BottomUp on an AI edge is AiNesting",
        );
    }
    let Some(x) = cx.person(Kind::Agent).await else {
        return;
    };
    cx.member(ai.id, &x).await;
    cx.ensure(
        !cx.is_effective(plain.id, &x).await,
        "AI rule: nothing flows from an AI group",
    );
}

async fn ai_switch(cx: &mut Cx<'_>) {
    let (Some(g), Some(child), Some(h), Some(ai)) = (
        cx.group("sw human").await,
        cx.group("sw child").await,
        cx.group("sw h").await,
        cx.ai_group("sw ai").await,
    ) else {
        return;
    };
    let (Some(p), Some(a)) = (cx.person(Kind::Human).await, cx.person(Kind::Agent).await) else {
        return;
    };
    cx.member(g.id, &p).await;
    cx.link(child.id, h.id, NestingMode::BottomUp).await;
    cx.member(ai.id, &a).await;
    let rev = cx.rev(g.id).await;
    cx.ensure(
        ai_nesting(err(&cx
            .g
            .update(
                g.id,
                rev,
                GroupPatch {
                    ai: Some(true),
                    ..GroupPatch::default()
                },
            )
            .await)),
        "update(ai): true on a group with a human is AiNesting",
    );
    let rev = cx.rev(h.id).await;
    cx.ensure(
        ai_nesting(err(&cx
            .g
            .update(
                h.id,
                rev,
                GroupPatch {
                    ai: Some(true),
                    ..GroupPatch::default()
                },
            )
            .await)),
        "update(ai): true on a group with an effective edge is AiNesting",
    );
    let rev = cx.rev(ai.id).await;
    cx.ensure(
        ai_nesting(err(&cx
            .g
            .update(
                ai.id,
                rev,
                GroupPatch {
                    ai: Some(false),
                    ..GroupPatch::default()
                },
            )
            .await)),
        "update(ai): false on an AI group with AI members is AiNesting",
    );
    let Some(empty) = cx.group("sw empty").await else {
        return;
    };
    cx.ensure(
        cx.g.update(
            empty.id,
            empty.revision,
            GroupPatch {
                ai: Some(true),
                ..GroupPatch::default()
            },
        )
        .await
        .is_ok_and(|g| g.ai),
        "update(ai): an empty group may become an AI group",
    );
}
