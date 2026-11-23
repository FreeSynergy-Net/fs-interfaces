//! Rules of `groups`, levels (a) and (b): scopes held by groups, the intervention with a reason, visible acts,
//! the end after an hour, no ownership through an intervention, the last `Project` scope stays.
//!
//! Pattern: Template method. The asking person at the start of the run is the installer (she grants the first
//! `Project` scope, see `ConformanceFixture::groups`).

use super::cx::{Cx, err};
use super::fixture::Kind;
use super::rules_owner::owned;
use crate::groups::{ActKind, AddonVerb, INTERVENTION_MS, Scope, role_owner};
use crate::groups::{Error, HistoryKind, Right};

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    let Some(v) = cx.gov else { return };
    let (Some(x), Some(y), Some(z), Some(owner_g)) = (
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
        owned(cx, "sc target").await,
    ) else {
        return;
    };
    // Each admin group is made by its own person (the creator is its owner and so a direct member): the installer
    // must not be a member of them, else she would hold the scope herself.
    cx.act(&x).await;
    let made_a = cx.group("sc admin a").await;
    cx.act(&y).await;
    let made_b = cx.group("sc admin b").await;
    cx.act_me().await;
    let (Some(admin_a), Some(admin_b)) = (made_a, made_b) else {
        return;
    };
    cx.ensure(
        matches!(
            err(&v.grant_scope(admin_a.id, Scope::Group(owner_g.id)).await),
            Some(Error::ScopeDenied(_))
        ),
        "grant_scope: Group(id) is never handed out (every group manages itself)",
    );
    cx.ensure(
        v.grant_scope(admin_a.id, Scope::Project).await.is_ok(),
        "grant_scope: the installer hands out the first Project scope",
    );
    cx.ensure(
        matches!(
            err(&v.grant_scope(admin_b.id, Scope::AllGroups).await),
            Some(Error::ScopeDenied(_))
        ),
        "grant_scope: the installer alone does not hand out more (only Project holders do)",
    );
    let me = cx.me.clone();
    if cx.act(&y).await {
        cx.ensure(
            matches!(
                err(&v.grant_scope(admin_b.id, Scope::AllGroups).await),
                Some(Error::ScopeDenied(_))
            ),
            "grant_scope: a stranger to the scope is refused",
        );
    }
    if cx.act(&x).await {
        cx.ensure(
            v.grant_scope(admin_b.id, Scope::AllGroups).await.is_ok(),
            "grant_scope: a Project holder hands out AllGroups to a group",
        );
    }
    // the intervention
    if cx.act(&y).await {
        cx.ensure(
            matches!(
                cx.g.add_member(owner_g.id, &x, None).await,
                Err(Error::Forbidden { .. })
            ),
            "without an intervention the scope does not work (no silent way)",
        );
        cx.ensure(
            matches!(
                err(&v
                    .open_intervention(owner_g.id, Scope::AllGroups, "  ")
                    .await),
                Some(Error::ReasonMissing)
            ),
            "open_intervention: the reason is mandatory",
        );
        cx.ensure(
            matches!(
                err(&v
                    .open_intervention(owner_g.id, Scope::Project, "too much")
                    .await),
                Some(Error::ScopeDenied(_))
            ),
            "open_intervention: a scope she does not hold is refused",
        );
        let iv = v
            .open_intervention(owner_g.id, Scope::AllGroups, "Zzconf: Mitglied fehlt")
            .await;
        cx.ensure(
            iv.as_ref().is_ok_and(|i| {
                i.actor == y
                    && i.admin_group == admin_b.id
                    && i.ends.millis() - i.opened.millis() == INTERVENTION_MS
            }),
            "open_intervention: actor, admin group and one hour",
        );
        cx.ensure(
            cx.g.add_member(owner_g.id, &x, None).await.is_ok(),
            "inside an intervention she may act",
        );
        cx.ensure(
            v.attach_addon(owner_g.id, super::rules_addon::other())
                .await
                .is_ok(),
            "inside an intervention she may attach an addon (and it is noted below)",
        );
        cx.ensure(
            matches!(
                err(&v.appoint_owner(owner_g.id, &x).await),
                Some(Error::NeedsOwner { .. })
            ),
            "an intervention is no ownership: appoint_owner needs the Owner level",
        );
        let Ok(iv) = iv else { return };
        cx.act_me().await;
        let seen = v.interventions(owner_g.id).await;
        cx.ensure(
            seen.as_ref().is_ok_and(|l| {
                l.iter().any(|i| {
                    i.id == iv.id
                        && i.reason.contains("Mitglied fehlt")
                        && i.acts.iter().any(|a| {
                            matches!(&a.kind, ActKind::History(HistoryKind::MemberAdded { .. }))
                        })
                })
            }),
            "interventions: the owner sees reason, actor and the act (every reader sees it)",
        );
        cx.ensure(
            seen.as_ref().is_ok_and(|l| {
                l.iter().any(|i| {
                    i.id == iv.id
                        && i.acts.iter().any(|a| {
                            matches!(&a.kind, ActKind::Addon { verb: AddonVerb::Attached, iri }
                                if *iri == super::rules_addon::other().iri())
                        })
                })
            }),
            "interventions: an addon attached in an intervention is an act of it (no silent way)",
        );
        cx.ensure(
            v.owners(owner_g.id).await.is_ok_and(|o| {
                o.len() == 1 && o[0].person == me && o[0].role == Some(role_owner())
            }),
            "an intervention does not change the owners",
        );
        cx.act(&y).await;
        cx.ensure(
            v.close_intervention(iv.id).await.is_ok(),
            "close_intervention: she closes her own",
        );
        cx.ensure(
            matches!(
                cx.g.add_member(owner_g.id, &z, None).await,
                Err(Error::Forbidden { .. })
            ),
            "after it is closed she cannot act",
        );
        cx.ensure(
            matches!(
                err(&v.close_intervention(iv.id).await),
                Some(Error::NoSuchIntervention(_))
            ),
            "close_intervention: twice is NoSuchIntervention",
        );
        let again = v
            .open_intervention(owner_g.id, Scope::AllGroups, "Zzconf zweite")
            .await;
        cx.ensure(again.is_ok(), "setup: open again");
        cx.fx.advance_clock(INTERVENTION_MS + 1000).await;
        cx.ensure(
            matches!(
                cx.g.add_member(owner_g.id, &z, None).await,
                Err(Error::Forbidden { .. })
            ),
            "the intervention ends by itself after an hour",
        );
    }
    cx.act(&x).await;
    cx.ensure(
        matches!(
            cx.g.my_rights(owner_g.id)
                .await
                .map(|r| r.contains(&Right::Manage)),
            Ok(false)
        ),
        "a Project holder has no rights without an intervention",
    );
    cx.ensure(
        v.scopes().await.is_ok_and(|s| s.len() == 2),
        "scopes: a holder sees the grants",
    );
    let Some(project) = v
        .scopes()
        .await
        .ok()
        .and_then(|s| s.into_iter().find(|g| g.scope == Scope::Project))
    else {
        return;
    };
    cx.ensure(
        matches!(
            err(&v.revoke_scope(project.id).await),
            Some(Error::LastScope)
        ),
        "revoke_scope: the last Project scope stays",
    );
    super::rules_operator::by_project_holder(cx, v, &x).await;
    cx.act(&x).await;
    super::rules_scope_ai::rules(cx, v, &x).await;
    cx.act_me().await;
}
