//! Rules: an asking AI on VISIBLE groups: it never agrees to a request and never gets the numbers of a nesting.
//!
//! Pattern: Template method (a fixed list of rules; the AI account is made through the fixture).

use super::cx::{Cx, err};
use super::fixture::Kind;
use super::rules_requests::asked;
use crate::groups_v1::{Error, NestingMode, RequestAnswer, RequestSide, RequestState, Right};

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    agrees_never(cx).await;
    effect_for_ai(cx).await;
}

/// The AI is no manager: a request on two visible groups is `Forbidden { Manage }` for it, and stays open.
async fn agrees_never(cx: &mut Cx<'_>) {
    let Some((_, parent, _, r)) = asked(cx, NestingMode::BottomUp).await else {
        return;
    };
    let Some(ai) = cx.person(Kind::Agent).await else {
        return;
    };
    // Even with a `Read` the AI was given expressly (allowed), it does not become a manager.
    cx.ensure(
        cx.g.grant(parent.id, &ai, Right::Read).await.is_ok(),
        "AI rule: an AI account may get Read on the asked group",
    );
    if !cx.act(&ai).await {
        return;
    }
    for answer in [RequestAnswer::Approve, RequestAnswer::Decline] {
        let got = cx.g.answer_nesting_request(r.id, r.revision, answer).await;
        cx.ensure(
            matches!(
                err(&got),
                Some(Error::Forbidden {
                    right: Right::Manage,
                    ..
                })
            ),
            "AI rule: an AI cannot agree to a request on a visible group, Forbidden { Manage }",
        );
    }
    cx.act_me().await;
    cx.ensure(
        cx.g.nesting_requests(parent.id, RequestSide::Incoming)
            .await
            .is_ok_and(|v| {
                v.iter()
                    .any(|x| x.id == r.id && x.state == RequestState::Pending)
            }),
        "AI rule: the request the AI could not answer is still open",
    );
}

/// `nesting_effect` for an AI: `NotFound` when a side is hidden from it, else `Forbidden { Manage }`; never numbers
/// (so a hidden group is never counted, and a visible one only for a manager).
async fn effect_for_ai(cx: &mut Cx<'_>) {
    let (Some(a), Some(b), Some(hidden)) = (
        cx.group("ef a").await,
        cx.group("ef b").await,
        cx.sensitive_group("ef hidden").await,
    ) else {
        return;
    };
    let (Some(x), Some(ai)) = (cx.person(Kind::Human).await, cx.person(Kind::Agent).await) else {
        return;
    };
    cx.member(a.id, &x).await;
    cx.member(hidden.id, &x).await;
    if !cx.act(&ai).await {
        return;
    }
    let visible = cx.g.nesting_effect(a.id, b.id, NestingMode::BottomUp).await;
    cx.ensure(
        matches!(err(&visible), Some(Error::Forbidden { right: Right::Manage, .. })),
        "AI rule: nesting_effect on two visible groups is Forbidden { Manage } for an AI, no numbers",
    );
    for (c, p) in [(hidden.id, b.id), (a.id, hidden.id), (hidden.id, hidden.id)] {
        let r = cx.g.nesting_effect(c, p, NestingMode::BottomUp).await;
        cx.ensure(
            matches!(err(&r), Some(Error::NotFound(_))),
            "AI rule: nesting_effect with a hidden side is NotFound for an AI, hidden groups are not counted",
        );
    }
    cx.act_me().await;
}
