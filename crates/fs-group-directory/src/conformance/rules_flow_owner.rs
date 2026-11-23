//! Rules of `groups`: ownership flows over NO edge, in no direction and in no mode (ADR 0095: ownership is the
//! group's own; only members and rights flow, see `rules_directions.rs`).
//!
//! Pattern: Template method (the same check for each of the three modes; a mode that leaks owners fails its row).

use super::cx::{Cx, err};
use super::fixture::Kind;
use super::rules_owner::owned;
use crate::groups::shared::ObjectId;
use crate::groups::shared::PersonRef;
use crate::groups::{Error, GroupGovernance, Level, NestingMode, Right, role_manager};

pub(crate) async fn rules(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    for (label, mode) in [
        ("bottom-up", NestingMode::BottomUp),
        ("top-down", NestingMode::TopDown),
        ("single", NestingMode::Single),
    ] {
        flows_no_owner(cx, v, label, mode).await;
        flows_no_rights(cx, v, label, mode).await;
    }
}

/// Whether `who` holds `Manage` at `g` (the answer of `may_do`, as the asker reads it).
async fn manages(cx: &Cx<'_>, g: ObjectId, who: &PersonRef) -> bool {
    cx.g.may_do(g, who, Right::Manage)
        .await
        .is_ok_and(|d| d.allowed)
}

/// What `who` is at `g` when she asks herself: the level and `Manage` in her own rights.
async fn level_and_rights(
    cx: &mut Cx<'_>,
    v: &dyn GroupGovernance,
    g: ObjectId,
    who: &PersonRef,
) -> Option<(Level, bool)> {
    if !cx.act(who).await {
        return None;
    }
    let level = v.my_level(g).await.ok();
    let rights = cx.g.my_rights(g).await.ok();
    cx.act_me().await;
    Some((level?, rights?.contains(&Right::Manage)))
}

/// Owner and manager of ONE group are nothing (no `Manage`, no `Owner` level, no owner appointments) in the group
/// on the other side of an edge, in BOTH directions and in every mode (ADR 0095 Pkt. 15).
async fn flows_no_rights(cx: &mut Cx<'_>, v: &dyn GroupGovernance, label: &str, mode: NestingMode) {
    let (Some(top), Some(low)) = (
        owned(cx, &format!("{label} rights top")).await,
        owned(cx, &format!("{label} rights low")).await,
    ) else {
        return;
    };
    let (Some(boss), Some(mgr_top), Some(lo), Some(mgr_low), Some(cand)) = (
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
    ) else {
        return;
    };
    cx.link(low.id, top.id, mode).await;
    cx.ensure(
        v.appoint_owner(top.id, &boss).await.is_ok()
            && cx
                .g
                .add_member(top.id, &mgr_top, Some(&role_manager()))
                .await
                .is_ok(),
        "setup: an owner and a manager of the top group",
    );
    // The low group gets its own owner and manager; the founder hands the low group over and stays a plain member.
    cx.member(low.id, &lo).await;
    cx.ensure(
        cx.g.add_member(low.id, &mgr_low, Some(&role_manager()))
            .await
            .is_ok(),
        "setup: a manager of the low group",
    );
    let me = cx.me.clone();
    let mine = super::rules_owner::owner_of(v, low.id, &me).await;
    let Some(mine) = mine else { return };
    cx.ensure(
        v.transfer_ownership(mine.id, mine.revision, &lo, false)
            .await
            .is_ok(),
        "setup: the low group is handed to its own owner",
    );
    for who in [&me, &boss, &mgr_top] {
        cx.ensure(
            !manages(cx, low.id, who).await,
            &format!("{label}: an owner or manager of the top group does not manage the low group (may_do)"),
        );
        let got = level_and_rights(cx, v, low.id, who).await;
        cx.ensure(
            got.is_some_and(|(level, manage)| level < Level::Manager && !manage),
            &format!("{label}: the top group's owner or manager is neither Owner nor Manager of the low group"),
        );
    }
    for who in [&lo, &mgr_low] {
        cx.ensure(
            !manages(cx, top.id, who).await,
            &format!("{label}: an owner or manager of the low group does not manage the top group (may_do)"),
        );
        let got = level_and_rights(cx, v, top.id, who).await;
        cx.ensure(
            got.is_some_and(|(level, manage)| level < Level::Manager && !manage),
            &format!("{label}: the low group's owner or manager is neither Owner nor Manager of the top group"),
        );
    }
    if cx.act(&boss).await {
        cx.ensure(
            matches!(
                err(&v.appoint_owner(low.id, &cand).await),
                Some(Error::NeedsOwner { .. })
            ),
            &format!("{label}: an owner of the top group appoints no owner of the low group"),
        );
        cx.act_me().await;
    }
    if cx.act(&lo).await {
        cx.ensure(
            matches!(
                err(&v.appoint_owner(top.id, &cand).await),
                Some(Error::NeedsOwner { .. } | Error::NotFound(_))
            ),
            &format!("{label}: an owner of the low group appoints no owner of the top group"),
        );
        cx.act_me().await;
    }
}

async fn flows_no_owner(cx: &mut Cx<'_>, v: &dyn GroupGovernance, label: &str, mode: NestingMode) {
    let (Some(top), Some(low)) = (
        owned(cx, &format!("{label} owner top")).await,
        owned(cx, &format!("{label} owner low")).await,
    ) else {
        return;
    };
    let Some(boss) = cx.person(Kind::Human).await else {
        return;
    };
    cx.ensure(
        v.appoint_owner(top.id, &boss).await.is_ok(),
        "setup: a second owner of the top group",
    );
    cx.link(low.id, top.id, mode).await;
    let on_low = v.owners(low.id).await.unwrap_or_default();
    cx.ensure(
        on_low.iter().all(|m| m.person != boss),
        &format!("{label}: an owner of the top group is not an owner of the low group"),
    );
    let on_top = v.owners(top.id).await.unwrap_or_default();
    cx.ensure(
        on_top.iter().any(|m| m.person == boss) && on_top.len() == 2,
        &format!("{label}: the owners of the top group stay its own"),
    );
}
