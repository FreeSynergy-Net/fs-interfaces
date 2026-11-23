//! Rules: requests between groups (both sides must agree), mode requests, withdrawing.
//!
//! Pattern: Template method (a fixed list of rules; a second person with `Manage` on one side only is seeded).

use super::cx::{Cx, err};
use super::fixture::Kind;
use crate::groups_v1::shared::Revision;
use crate::groups_v1::{
    Error, NestOutcome, NestingMode, NestingRequest, NestingRequestKind, NestingSide,
    RequestAnswer, RequestSide, RequestState, Right,
};

/// A request made by the person who manages only the child side; returns the group ids and the request.
pub(crate) async fn asked(
    cx: &mut Cx<'_>,
    mode: NestingMode,
) -> Option<(
    crate::groups_v1::Group,
    crate::groups_v1::Group,
    crate::groups_v1::shared::PersonRef,
    NestingRequest,
)> {
    let parent = cx.group("rq parent").await?;
    let h2 = cx.person(Kind::Human).await?;
    if !cx.act(&h2).await {
        return None;
    }
    let child = cx.group("rq child").await?;
    let out = cx.g.request_nesting(child.id, parent.id, mode).await;
    cx.act_me().await;
    if let Ok(NestOutcome::Requested(r)) = out {
        return Some((child, parent, h2, r));
    }
    cx.ensure(
        false,
        "request: one side's Manage only makes a request to the other side",
    );
    None
}

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    let Some((child, parent, h2, r)) = asked(cx, NestingMode::BottomUp).await else {
        return;
    };
    cx.ensure(
        r.state == RequestState::Pending
            && r.asked == NestingSide::AsParent
            && r.requested_by == h2
            && r.kind
                == NestingRequestKind::Link {
                    mode: NestingMode::BottomUp,
                },
        "request: pending, asks the parent side, names the requester and the mode",
    );
    cx.ensure(
        cx.g.nestings(child.id, NestingSide::AsChild)
            .await
            .is_ok_and(|e| e.is_empty()),
        "request: no edge before both agreed",
    );
    let incoming =
        cx.g.nesting_requests(parent.id, RequestSide::Incoming)
            .await
            .unwrap_or_default();
    cx.ensure(
        incoming.iter().any(|x| x.id == r.id),
        "nesting_requests: the asked side sees it as incoming",
    );
    cx.ensure(
        cx.g.nesting_requests(parent.id, RequestSide::Outgoing)
            .await
            .is_ok_and(|v| v.is_empty()),
        "nesting_requests: not as outgoing",
    );
    cx.ensure(matches!(err(&cx.g.request_nesting(child.id, parent.id, NestingMode::Single).await), Some(Error::AlreadyThere(id)) if *id == r.id), "request: a second one for the pair is AlreadyThere with the request id");
    if cx.act(&h2).await {
        cx.ensure(
            cx.g.nesting_requests(child.id, RequestSide::Outgoing)
                .await
                .is_ok_and(|v| v.iter().any(|x| x.id == r.id)),
            "nesting_requests: the requesting side sees it as outgoing",
        );
        cx.ensure(
            matches!(
                err(&cx
                    .g
                    .nesting_requests(parent.id, RequestSide::Incoming)
                    .await),
                Some(Error::Forbidden {
                    right: Right::Manage,
                    ..
                })
            ),
            "nesting_requests: needs Manage at the group",
        );
        cx.ensure(
            matches!(
                err(&cx
                    .g
                    .answer_nesting_request(r.id, r.revision, RequestAnswer::Approve)
                    .await),
                Some(Error::Forbidden {
                    right: Right::Manage,
                    ..
                })
            ),
            "answer: the requester does not manage the asked side, Forbidden",
        );
        cx.act_me().await;
    }
    cx.ensure(
        matches!(
            err(&cx
                .g
                .answer_nesting_request(r.id, Revision(r.revision.0 + 9), RequestAnswer::Approve)
                .await),
            Some(Error::Changed { .. })
        ),
        "answer: an old revision is Changed",
    );
    let done =
        cx.g.answer_nesting_request(r.id, r.revision, RequestAnswer::Approve)
            .await;
    cx.ensure(
        done.as_ref().is_ok_and(|d| {
            d.state == RequestState::Approved && d.answered_by.is_some() && d.answered.is_some()
        }),
        "answer: Approve closes the request",
    );
    let edges =
        cx.g.nestings(child.id, NestingSide::AsChild)
            .await
            .unwrap_or_default();
    cx.ensure(
        edges.len() == 1
            && edges[0].mode == NestingMode::BottomUp
            && edges[0].approved_by.is_some()
            && edges[0].linked_by == h2,
        "answer: the edge exists, with who asked and who agreed",
    );
    let again =
        cx.g.answer_nesting_request(
            r.id,
            done.map_or(r.revision, |d| d.revision),
            RequestAnswer::Decline,
        )
        .await;
    cx.ensure(
        matches!(
            err(&again),
            Some(Error::RequestClosed(RequestState::Approved))
        ),
        "answer: a closed request is RequestClosed",
    );
    super::rules_requests_more::rules(cx).await;
}
