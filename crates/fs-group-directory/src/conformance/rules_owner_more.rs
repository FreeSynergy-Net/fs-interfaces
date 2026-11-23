//! Rules of `groups`, owners (part 2): candidates, transfer, withheld reads, owners of objects, edge modes.
//!
//! Pattern: Template method (a fixed list of rules; each one is a mutant killer).

use super::cx::{Cx, err};
use super::fixture::Kind;
use super::rules_owner::{owned, owned_from, owner_of};
use crate::groups::shared::Revision;
use crate::groups::{Error, NestOutcome, NestingMode, Visibility};
use crate::groups::{GroupGovernance, role_owner};
use crate::ids::group_ref;

pub(super) async fn rules(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    candidates(cx, v).await;
    transfer(cx, v).await;
    withheld(cx, v).await;
    object_owner(cx, v).await;
    modes(cx, v).await;
}

async fn candidates(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    let Some(g) = owned(cx, v, "own cand").await else {
        return;
    };
    let (Some(bot), Some(unk), Some(h)) = (
        cx.person(Kind::Agent).await,
        cx.person(Kind::Unknown).await,
        cx.person(Kind::Human).await,
    ) else {
        return;
    };
    cx.ensure(
        matches!(
            err(&v.appoint_owner(g.id, &bot).await),
            Some(Error::AgentNotAdmin)
        ),
        "appoint_owner: an AI account is never an owner",
    );
    cx.ensure(
        matches!(
            err(&v.appoint_owner(g.id, &unk).await),
            Some(Error::AiStatusUnknown)
        ),
        "appoint_owner: an unknown account is never an owner",
    );
    cx.ensure(
        matches!(
            err(&v.appoint_owner(g.id, &group_ref(g.id)).await),
            Some(Error::WrongArt { .. })
        ),
        "appoint_owner: a group is never an owner",
    );
    cx.ensure(
        v.owners(g.id).await.is_ok_and(|o| o.len() == 1),
        "owners: the refused ones did not get in",
    );
    cx.ensure(
        v.appoint_owner(g.id, &h).await.is_ok(),
        "appoint_owner: a human who is not yet a member joins as owner",
    );
    cx.ensure(
        cx.g.members(g.id, None, 100).await.is_ok_and(|p| {
            p.items
                .iter()
                .any(|m| m.person == h && m.role == Some(role_owner()))
        }),
        "appoint_owner: the owner is a direct member (a person reference, nothing else)",
    );
}

async fn transfer(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    let Some(g) = owned(cx, v, "own transfer").await else {
        return;
    };
    let (Some(p), Some(q)) = (cx.person(Kind::Human).await, cx.person(Kind::Human).await) else {
        return;
    };
    let me = cx.me.clone();
    let Some(mine) = owner_of(v, g.id, &me).await else {
        return;
    };
    cx.ensure(
        matches!(
            err(&v
                .transfer_ownership(mine.id, mine.revision, &p, false)
                .await),
            Some(Error::NotDirectMember { .. })
        ),
        "transfer_ownership: the target must be a direct member",
    );
    cx.member(g.id, &p).await;
    cx.ensure(
        matches!(
            err(&v
                .transfer_ownership(mine.id, Revision(mine.revision.0 + 9), &p, false)
                .await),
            Some(Error::Changed { .. })
        ),
        "transfer_ownership: a stale revision is Changed",
    );
    cx.ensure(
        v.transfer_ownership(mine.id, mine.revision, &p, true)
            .await
            .is_ok(),
        "transfer_ownership: keep = true gives co-ownership",
    );
    cx.ensure(
        v.owners(g.id).await.is_ok_and(|o| o.len() == 2),
        "transfer_ownership: both are owners",
    );
    cx.member(g.id, &q).await;
    if cx.act(&p).await {
        let Some(pm) = owner_of(v, g.id, &p).await else {
            return;
        };
        cx.ensure(
            matches!(
                err(&v
                    .transfer_ownership(mine.id, mine.revision, &q, false)
                    .await),
                Some(Error::NotYourMembership(_))
            ),
            "transfer_ownership: the actor is the connection, not the membership named",
        );
        cx.ensure(
            v.transfer_ownership(pm.id, pm.revision, &q, false)
                .await
                .is_ok(),
            "transfer_ownership: the owner hands over to q",
        );
        cx.act_me().await;
    }
    cx.ensure(
        v.owners(g.id)
            .await
            .is_ok_and(|o| o.iter().any(|m| m.person == q) && !o.iter().any(|m| m.person == p)),
        "transfer_ownership: q is an owner and p handed over",
    );
}

async fn withheld(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    let mut d = cx.draft("own sens");
    (d.sensitive, d.visibility) = (true, Visibility::Members);
    let Some(g) = owned_from(cx, v, d).await else {
        return;
    };
    let Some(s) = cx.person(Kind::Human).await else {
        return;
    };
    cx.ensure(
        v.owners(g.id).await.is_ok_and(|o| o.len() == 1),
        "owners: a reader sees them, also in a sensitive group",
    );
    if cx.act(&s).await {
        cx.ensure(
            matches!(err(&v.owners(g.id).await), Some(Error::Withheld { .. })),
            "owners: without insight it is Withheld, never an empty list",
        );
        cx.ensure(
            matches!(err(&v.addons(g.id).await), Some(Error::Withheld { .. })),
            "addons: without insight it is Withheld",
        );
        cx.ensure(
            matches!(
                err(&v.interventions(g.id).await),
                Some(Error::Withheld { .. })
            ),
            "interventions: without insight it is Withheld",
        );
        cx.ensure(
            matches!(
                err(&v.resolve_owner(&group_ref(g.id)).await),
                Some(Error::Withheld { .. })
            ),
            "resolve_owner: a group she may not read is Withheld",
        );
        cx.act_me().await;
    }
}

async fn object_owner(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    let Some(g) = owned(cx, v, "own object").await else {
        return;
    };
    let Some(p) = cx.person(Kind::Human).await else {
        return;
    };
    cx.ensure(
        v.appoint_owner(g.id, &p).await.is_ok(),
        "setup: second owner",
    );
    let me = cx.me.clone();
    cx.ensure(
        v.resolve_owner(&p).await == Ok(vec![p.clone()]),
        "resolve_owner: a person owns as herself",
    );
    cx.ensure(
        v.resolve_owner(&group_ref(g.id))
            .await
            .is_ok_and(|o| o.len() == 2 && o.contains(&me) && o.contains(&p)),
        "resolve_owner: an object owned by a group resolves to the group's owners",
    );
    let nowhere = group_ref(crate::ids::object_id(9_999_999));
    cx.ensure(
        matches!(
            err(&v.resolve_owner(&nowhere).await),
            Some(Error::NotFound(_))
        ),
        "resolve_owner: an unknown group is NotFound",
    );
}

async fn modes(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    let (mut dt, mut db) = (cx.draft("mode top"), cx.draft("mode bottom"));
    (dt.default_mode, db.default_mode) = (NestingMode::TopDown, NestingMode::BottomUp);
    let (Some(top), Some(bottom), Some(child)) = (
        owned_from(cx, v, dt).await,
        owned_from(cx, v, db).await,
        owned(cx, v, "mode child").await,
    ) else {
        return;
    };
    let edge = |o: Result<NestOutcome, Error>| match o {
        Ok(NestOutcome::Linked(e)) => Some(e),
        _ => None,
    };
    let e1 = edge(v.nest_default(child.id, top.id).await);
    let e2 = edge(v.nest_default(child.id, bottom.id).await);
    cx.ensure(
        e1.as_ref().is_some_and(|e| e.mode == NestingMode::TopDown),
        "nest_default: the edge takes the default_mode of the upper group (TopDown)",
    );
    cx.ensure(
        e2.as_ref().is_some_and(|e| e.mode == NestingMode::BottomUp),
        "nest_default: ... and BottomUp for another parent (the mode is per edge)",
    );
    if let Some(e) = e1 {
        cx.ensure(
            cx.g.change_nesting_mode(e.id, e.revision, NestingMode::Single)
                .await
                .is_ok(),
            "setup: change one edge",
        );
        cx.ensure(
            cx.fresh(top.id)
                .await
                .is_some_and(|t| t.default_mode == NestingMode::TopDown),
            "default_mode of the group stays when an edge changes",
        );
        cx.ensure(
            cx.g.nestings(child.id, crate::groups::NestingSide::AsChild)
                .await
                .is_ok_and(|n| {
                    n.iter().any(|x| x.mode == NestingMode::BottomUp)
                        && n.iter().any(|x| x.mode == NestingMode::Single)
                }),
            "the other edge of the same child keeps its own mode",
        );
    }
}
