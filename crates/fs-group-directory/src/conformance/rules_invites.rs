//! Rules: invitations (open, answered, revoked, expired).
//!
//! Pattern: Template method (a fixed list of rules; each rule seeds its own data through the trait).

use super::cx::{Cx, err};
use super::fixture::Kind;
use crate::groups::shared::Revision;
use crate::groups::{
    Error, Field, InForce, InvitationAnswer, InvitationOutcome, InvitationState, ProblemKind, Right,
};
use crate::ids::group_ref;

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    invitations(cx).await;
    invitation_ends(cx).await;
}

async fn invitations(cx: &mut Cx<'_>) {
    let (Some(g), Some(p)) = (cx.group("inv").await, cx.person(Kind::Human).await) else {
        return;
    };
    let Ok(inv) = cx.g.invite(g.id, &p, None, None).await else {
        cx.ensure(false, "invite: the manager invites");
        return;
    };
    cx.ensure(
        inv.state == InvitationState::Pending && inv.invitee == p && inv.group == g.id,
        "invite: pending, with the invitee",
    );
    cx.ensure(matches!(err(&cx.g.invite(g.id, &p, None, None).await), Some(Error::AlreadyThere(id)) if *id == inv.id), "invite: an open invitation again is AlreadyThere");
    cx.ensure(
        matches!(
            err(&cx.g.invite(g.id, &group_ref(g.id), None, None).await),
            Some(Error::WrongArt { .. })
        ),
        "invite: a group is WrongArt",
    );
    let past = crate::groups::shared::Utc::from_millis(1);
    let bad = cx.g.invite(g.id, &p, None, Some(past)).await;
    cx.ensure(matches!(err(&bad), Some(Error::Invalid(v)) if v.iter().any(|x| x.field == Field::Expires && x.problem == ProblemKind::InPast)), "invite: an expiry in the past is Invalid");
    cx.ensure(
        cx.g.invitations(g.id).await.is_ok_and(|v| v.len() == 1),
        "invitations: the group's list",
    );
    if !cx.act(&p).await {
        return;
    }
    cx.ensure(
        cx.g.my_invitations()
            .await
            .is_ok_and(|v| v.iter().any(|i| i.id == inv.id)),
        "my_invitations: the invitee sees it",
    );
    cx.ensure(
        matches!(
            err(&cx.g.invitations(g.id).await),
            Some(Error::Forbidden {
                right: Right::Read,
                ..
            })
        ),
        "invitations: without Read it is Forbidden",
    );
    cx.ensure(
        matches!(
            err(&cx
                .g
                .answer_invitation(
                    inv.id,
                    Revision(inv.revision.0 + 9),
                    InvitationAnswer::Accept
                )
                .await),
            Some(Error::Changed { .. })
        ),
        "answer_invitation: an old revision is Changed",
    );
    let joined =
        cx.g.answer_invitation(inv.id, inv.revision, InvitationAnswer::Accept)
            .await;
    let Ok(InvitationOutcome::Joined(m)) = joined else {
        cx.ensure(false, "answer_invitation: Accept gives a membership");
        return;
    };
    cx.ensure(
        m.person == p && m.from_invitation == Some(inv.id) && m.in_force == InForce::Yes,
        "answer_invitation: the membership names its invitation",
    );
    cx.ensure(
        cx.g.my_invitations().await.is_ok_and(|v| v.is_empty()),
        "my_invitations: an answered one is gone",
    );
    let again =
        cx.g.answer_invitation(inv.id, inv.revision, InvitationAnswer::Accept)
            .await;
    cx.ensure(
        err(&again).is_some(),
        "answer_invitation: answering twice fails",
    );
    cx.act_me().await;
    cx.ensure(
        cx.g.invitations(g.id)
            .await
            .is_ok_and(|v| v[0].state == InvitationState::Accepted && v[0].answered.is_some()),
        "invitations: Accepted with the answer time",
    );
}

async fn invitation_ends(cx: &mut Cx<'_>) {
    let (Some(g), Some(p), Some(q), Some(r)) = (
        cx.group("inv2").await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
    ) else {
        return;
    };
    let (Ok(ip), Ok(iq)) = (
        cx.g.invite(g.id, &p, None, None).await,
        cx.g.invite(g.id, &q, None, None).await,
    ) else {
        return;
    };
    if cx.act(&r).await {
        cx.ensure(
            matches!(
                err(&cx
                    .g
                    .answer_invitation(ip.id, ip.revision, InvitationAnswer::Accept)
                    .await),
                Some(Error::NotInvitee)
            ),
            "answer_invitation: only the invitee, else NotInvitee",
        );
        cx.ensure(
            matches!(
                err(&cx.g.revoke_invitation(ip.id, ip.revision).await),
                Some(Error::Forbidden {
                    right: Right::RemoveMembers,
                    ..
                })
            ),
            "revoke_invitation: a stranger is Forbidden",
        );
    }
    if cx.act(&p).await {
        let d =
            cx.g.answer_invitation(ip.id, ip.revision, InvitationAnswer::Decline)
                .await;
        cx.ensure(matches!(d, Ok(InvitationOutcome::Declined(ref i)) if i.state == InvitationState::Declined), "answer_invitation: Decline");
        cx.ensure(
            cx.g.groups_of(&p).await.is_ok_and(|v| v.is_empty()),
            "answer_invitation: Decline makes no membership",
        );
    }
    cx.act_me().await;
    let rv = cx.g.revoke_invitation(iq.id, iq.revision).await;
    cx.ensure(
        rv.as_ref()
            .is_ok_and(|i| i.state == InvitationState::Revoked),
        "revoke_invitation: the inviter revokes",
    );
    cx.ensure(
        matches!(
            err(&cx
                .g
                .revoke_invitation(iq.id, rv.map_or(iq.revision, |i| i.revision))
                .await),
            Some(Error::InvitationClosed(InvitationState::Revoked))
        ),
        "revoke_invitation: twice is InvitationClosed",
    );
    // The clock: an invitation past its date reads as Expired and cannot be accepted.
    let Some(h) = cx.group("inv3").await else {
        return;
    };
    let Some(s) = cx.person(Kind::Human).await else {
        return;
    };
    let soon = crate::groups::shared::Utc::from_millis(h.created.millis() + 3_600_000);
    let Ok(ie) = cx.g.invite(h.id, &s, None, Some(soon)).await else {
        return;
    };
    if !cx.fx.advance_clock(2 * 3_600_000).await {
        cx.skip("invitations: Expired (the fixture cannot move the clock)");
        return;
    }
    cx.ensure(
        cx.g.invitations(h.id)
            .await
            .is_ok_and(|v| v[0].state == InvitationState::Expired),
        "invitations: past its date it reads as Expired",
    );
    if cx.act(&s).await {
        let r =
            cx.g.answer_invitation(ie.id, ie.revision, InvitationAnswer::Accept)
                .await;
        cx.ensure(
            matches!(
                err(&r),
                Some(Error::Expired { .. } | Error::InvitationClosed(InvitationState::Expired))
            ),
            "answer_invitation: an expired one cannot be accepted",
        );
        cx.act_me().await;
    }
}
