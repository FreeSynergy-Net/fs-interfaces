//! Rules: declining, withdrawing and mode requests.
//!
//! Pattern: Template method (a fixed list of rules; each rule seeds its own data through the trait).

use super::cx::{Cx, err};
use super::fixture::Kind;
use super::rules_requests::asked;
use crate::groups_v1::{
    Error, NestOutcome, NestingMode, NestingRequestKind, NestingSide, RequestAnswer, RequestSide,
    RequestState,
};

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    decline_and_withdraw(cx).await;
    mode_requests(cx).await;
}

async fn decline_and_withdraw(cx: &mut Cx<'_>) {
    let Some((_, parent, _, r)) = asked(cx, NestingMode::TopDown).await else {
        return;
    };
    let d =
        cx.g.answer_nesting_request(r.id, r.revision, RequestAnswer::Decline)
            .await;
    cx.ensure(
        d.is_ok_and(|d| d.state == RequestState::Declined),
        "answer: Decline closes the request",
    );
    cx.ensure(
        cx.g.nestings(parent.id, NestingSide::AsParent)
            .await
            .is_ok_and(|e| e.is_empty()),
        "answer: Decline makes no edge",
    );
    let Some((_, _, h3, r2)) = asked(cx, NestingMode::Single).await else {
        return;
    };
    if let Some(stranger) = cx.person(Kind::Human).await {
        if cx.act(&stranger).await {
            cx.ensure(
                matches!(
                    err(&cx.g.withdraw_nesting_request(r2.id, r2.revision).await),
                    Some(Error::Forbidden { .. })
                ),
                "withdraw: a stranger is Forbidden",
            );
            cx.act_me().await;
        }
    }
    if cx.act(&h3).await {
        let w = cx.g.withdraw_nesting_request(r2.id, r2.revision).await;
        cx.ensure(
            w.as_ref().is_ok_and(|w| w.state == RequestState::Withdrawn),
            "withdraw: the requester withdraws",
        );
        cx.ensure(
            matches!(
                err(&cx
                    .g
                    .withdraw_nesting_request(r2.id, w.map_or(r2.revision, |w| w.revision))
                    .await),
                Some(Error::RequestClosed(RequestState::Withdrawn))
            ),
            "withdraw: twice is RequestClosed",
        );
        cx.act_me().await;
    }
    let Some((c3, p3, h4, r3)) = asked(cx, NestingMode::BottomUp).await else {
        return;
    };
    let rev = cx.rev(p3.id).await;
    cx.ensure(
        cx.g.delete(p3.id, rev).await.is_ok(),
        "delete: a group with an open request",
    );
    if cx.act(&h4).await {
        let seen =
            cx.g.nesting_requests(c3.id, RequestSide::Outgoing)
                .await
                .unwrap_or_default();
        cx.ensure(
            seen.iter()
                .any(|x| x.id == r3.id && x.state == RequestState::Withdrawn),
            "delete: the open requests of the group are Withdrawn",
        );
        cx.act_me().await;
    }
}

/// A mode change that needs both sides: a request; `Changed` when the edge moved meanwhile; a dissolved edge
/// withdraws its open requests.
async fn mode_requests(cx: &mut Cx<'_>) {
    let Some((child, parent, h2, r)) = asked(cx, NestingMode::Single).await else {
        return;
    };
    let _ =
        cx.g.answer_nesting_request(r.id, r.revision, RequestAnswer::Approve)
            .await;
    let Some(edge) =
        cx.g.nestings(child.id, NestingSide::AsChild)
            .await
            .ok()
            .and_then(|v| v.into_iter().next())
    else {
        return;
    };
    if !cx.act(&h2).await {
        return;
    }
    let up =
        cx.g.change_nesting_mode(edge.id, edge.revision, NestingMode::BottomUp)
            .await;
    cx.act_me().await;
    let Ok(NestOutcome::Requested(req)) = up else {
        cx.ensure(
            false,
            "mode: strengthening with one side's Manage is a request",
        );
        return;
    };
    cx.ensure(
        matches!(
            req.kind,
            NestingRequestKind::ChangeMode {
                from: NestingMode::Single,
                to: NestingMode::BottomUp,
                ..
            }
        ),
        "mode: the request names from and to",
    );
    cx.ensure(
        cx.g.nestings(child.id, NestingSide::AsChild)
            .await
            .is_ok_and(|e| e[0].mode == NestingMode::Single),
        "mode: nothing changes before the other side agrees",
    );
    let moved =
        cx.g.change_nesting_mode(edge.id, edge.revision, NestingMode::TopDown)
            .await;
    cx.ensure(
        matches!(err(&moved), Some(Error::AlreadyThere(id)) if *id == req.id),
        "mode: a second request for the edge is AlreadyThere with its id",
    );
    let rev =
        cx.g.nestings(child.id, NestingSide::AsChild)
            .await
            .ok()
            .and_then(|v| v.first().map(|e| e.revision))
            .unwrap_or(edge.revision);
    cx.ensure(
        matches!(
            cx.g.change_nesting_mode(edge.id, rev, NestingMode::TopDown)
                .await,
            Err(Error::AlreadyThere(_))
        ),
        "mode: still the one open request",
    );
    let ok =
        cx.g.answer_nesting_request(req.id, req.revision, RequestAnswer::Approve)
            .await;
    cx.ensure(
        ok.is_ok_and(|a| a.state == RequestState::Approved),
        "mode: the other side approves",
    );
    cx.ensure(
        cx.g.nestings(child.id, NestingSide::AsChild)
            .await
            .is_ok_and(|e| e[0].mode == NestingMode::BottomUp),
        "mode: the edge has the new mode",
    );
    // An open mode request is withdrawn when its edge is dissolved.
    if !cx.act(&h2).await {
        return;
    }
    let e2 =
        cx.g.nestings(child.id, NestingSide::AsChild)
            .await
            .ok()
            .and_then(|v| v.into_iter().next());
    let Some(e2) = e2 else { return };
    let weak =
        cx.g.change_nesting_mode(e2.id, e2.revision, NestingMode::Single)
            .await;
    let Ok(NestOutcome::Linked(weak)) = weak else {
        return;
    };
    let again =
        cx.g.change_nesting_mode(weak.id, weak.revision, NestingMode::TopDown)
            .await;
    cx.act_me().await;
    let Ok(NestOutcome::Requested(open)) = again else {
        return;
    };
    let _ = cx.g.remove_nesting(weak.id, weak.revision).await;
    let seen =
        cx.g.nesting_requests(parent.id, RequestSide::Incoming)
            .await
            .unwrap_or_default();
    cx.ensure(
        seen.iter()
            .any(|x| x.id == open.id && x.state == RequestState::Withdrawn),
        "remove_nesting: the open requests of the edge are Withdrawn",
    );
}
