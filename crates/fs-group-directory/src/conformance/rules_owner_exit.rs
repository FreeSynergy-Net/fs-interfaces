//! Rules of `groups`: what a person keeps when she stops being an owner or manager (ADR 0095 Pkt. 4, 6, 8, 10, 11),
//! and the guard of the last manager when a role is changed.
//!
//! Pattern: Template method (a fixed list of rules; each one is a mutant killer: a provider that gives the creator a
//! hidden personal `Manage` fails the first two, one without the guard in `set_role` fails the third).

use super::cx::{Cx, err};
use super::fixture::Kind;
use super::rules_owner::{owned, owner_of};
use crate::groups::shared::{ObjectId, PersonRef};
use crate::groups::{Error, GroupGovernance, NestingMode, Right, role_manager};

pub(super) async fn rules(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    handed_over_and_left(cx, v).await;
    demoted(cx, v).await;
    last_manager_in_set_role(cx).await;
    for (label, mode) in [
        ("bottom-up", NestingMode::BottomUp),
        ("top-down", NestingMode::TopDown),
        ("single", NestingMode::Single),
    ] {
        last_owner_stays(cx, v, label, mode).await;
    }
}

/// The last owner cannot leave (`remove_member`) or step down (`set_role`) even when a manager exists; with a second
/// owner she may (ADR 0095 Pkt. 8 and 11). The group sits on an edge of every mode (the member paths do not care).
async fn last_owner_stays(
    cx: &mut Cx<'_>,
    v: &dyn GroupGovernance,
    label: &str,
    mode: NestingMode,
) {
    let (Some(g), Some(other)) = (
        owned(cx, &format!("{label} last owner")).await,
        owned(cx, &format!("{label} last owner edge")).await,
    ) else {
        return;
    };
    cx.link(g.id, other.id, mode).await;
    let (Some(mgr), Some(second)) = (cx.person(Kind::Human).await, cx.person(Kind::Human).await)
    else {
        return;
    };
    cx.ensure(
        cx.g.add_member(g.id, &mgr, Some(&role_manager()))
            .await
            .is_ok(),
        "setup: a manager next to the last owner",
    );
    cx.member(g.id, &second).await;
    let me = cx.me.clone();
    let Some(mine) = owner_of(v, g.id, &me).await else {
        return;
    };
    cx.ensure(
        matches!(
            err(&cx.g.remove_member(mine.id, mine.revision).await),
            Some(Error::LastManager)
        ),
        &format!("{label}: the last owner cannot leave while a manager exists (remove_member)"),
    );
    cx.ensure(
        matches!(
            err(&cx.g.set_role(mine.id, mine.revision, None).await),
            Some(Error::LastManager)
        ),
        &format!("{label}: the last owner cannot step down while a manager exists (set_role)"),
    );
    cx.ensure(
        v.owners(g.id).await.is_ok_and(|o| o.len() == 1),
        &format!("{label}: she is still the only owner"),
    );
    cx.ensure(
        v.appoint_owner(g.id, &second).await.is_ok(),
        "setup: a second owner",
    );
    let Some(mine) = owner_of(v, g.id, &me).await else {
        return;
    };
    cx.ensure(
        cx.g.set_role(mine.id, mine.revision, None).await.is_ok(),
        &format!("{label}: with a second owner the first may step down (set_role)"),
    );
}

/// Whether `who` holds `Manage` at `g` now.
async fn manages(cx: &Cx<'_>, g: ObjectId, who: &PersonRef) -> bool {
    cx.g.may_do(g, who, Right::Manage)
        .await
        .is_ok_and(|d| d.allowed)
}

/// The founder hands the group over (`keep` false) and leaves: she holds NO right any more (no hidden personal grant).
async fn handed_over_and_left(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    let Some(g) = owned(cx, "exit hand over").await else {
        return;
    };
    let Some(next) = cx.person(Kind::Human).await else {
        return;
    };
    let me = cx.me.clone();
    cx.member(g.id, &next).await;
    let Some(mine) = owner_of(v, g.id, &me).await else {
        return;
    };
    cx.ensure(
        cx.g.grants(g.id).await.is_ok_and(|l| l.is_empty()),
        "create: the founder has no personal grant, only the owner role",
    );
    cx.ensure(
        v.transfer_ownership(mine.id, mine.revision, &next, false)
            .await
            .is_ok(),
        "setup: the founder hands the group over",
    );
    cx.ensure(
        !manages(cx, g.id, &me).await,
        "transfer_ownership: after handing over the former owner holds no Manage",
    );
    cx.ensure(
        cx.g.my_rights(g.id)
            .await
            .is_ok_and(|r| !r.contains(&Right::Manage)),
        "transfer_ownership: after handing over her own rights hold no Manage",
    );
    let rows =
        cx.g.members(g.id, None, 100)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    let Some(row) = rows.into_iter().find(|m| m.person == me) else {
        return;
    };
    cx.ensure(
        cx.g.remove_member(row.id, row.revision).await.is_ok(),
        "setup: the former owner leaves",
    );
    if cx.act(&next).await {
        cx.ensure(
            !manages(cx, g.id, &me).await,
            "after handing over and leaving the former owner holds no Manage (no hidden grant)",
        );
        cx.ensure(
            v.owners(g.id)
                .await
                .is_ok_and(|o| o.len() == 1 && o[0].person == next),
            "after handing over and leaving the new owner is the only owner",
        );
        cx.act_me().await;
    }
}

/// An owner demoted by another owner keeps nothing either.
async fn demoted(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    let Some(g) = owned(cx, "exit demote").await else {
        return;
    };
    let Some(other) = cx.person(Kind::Human).await else {
        return;
    };
    let me = cx.me.clone();
    cx.ensure(
        v.appoint_owner(g.id, &other).await.is_ok(),
        "setup: a second owner",
    );
    let Some(mine) = owner_of(v, g.id, &me).await else {
        return;
    };
    if !cx.act(&other).await {
        return;
    }
    cx.ensure(
        v.demote_owner(mine.id, mine.revision).await.is_ok(),
        "setup: the second owner demotes the founder",
    );
    cx.ensure(
        !manages(cx, g.id, &me).await,
        "demote_owner: a demoted owner holds no Manage",
    );
    cx.act_me().await;
}

/// `set_role` that leaves a group without any `Manage` holder is `LastManager`; while an owner remains, a manager may
/// step down (the owner role is itself `Manage`, ADR 0095 Pkt. 8).
async fn last_manager_in_set_role(cx: &mut Cx<'_>) {
    let Some(g) = owned(cx, "exit last manager").await else {
        return;
    };
    let (Some(boss), Some(helper)) = (cx.person(Kind::Human).await, cx.person(Kind::Human).await)
    else {
        return;
    };
    let me = cx.me.clone();
    let manager = role_manager();
    let (Ok(b), Ok(h)) = (
        cx.g.add_member(g.id, &boss, Some(&manager)).await,
        cx.g.add_member(g.id, &helper, Some(&manager)).await,
    ) else {
        cx.ensure(false, "setup: two managers");
        return;
    };
    cx.ensure(
        cx.g.set_role(h.id, h.revision, None).await.is_ok(),
        "set_role: a manager steps down while an owner remains",
    );
    // No effective owner is left (the founder is marked as an AI after the fact): `boss` is the only manager.
    if !cx.fx.mark(&me, Kind::Agent).await {
        cx.skip("set_role: the last manager (the fixture cannot mark a person)");
        return;
    }
    if cx.act(&boss).await {
        cx.ensure(
            matches!(
                err(&cx.g.set_role(b.id, b.revision, None).await),
                Some(Error::LastManager)
            ),
            "set_role: the last manager cannot step down (LastManager)",
        );
        cx.ensure(
            manages(cx, g.id, &boss).await,
            "set_role: after the refusal she still manages",
        );
    }
    cx.fx.mark(&me, Kind::Human).await;
    cx.act_me().await;
}
