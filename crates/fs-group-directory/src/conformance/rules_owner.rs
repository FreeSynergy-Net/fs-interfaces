//! Rules of `groups`: owners, levels, the guard of the last owner, who acts, what is withheld, owners of objects,
//! the edge mode and `default_mode`.
//!
//! Pattern: Template method (a fixed list of rules; each one is a mutant killer: break the rule and a check fails).

use super::cx::{Cx, err};
use super::fixture::Kind;
use crate::groups::shared::{ObjectId, PersonRef};
use crate::groups::{Error, Group, GroupDraft, Membership, Right};
use crate::groups::{GroupGovernance, Level, role_owner};

/// A group the asker created with `create` (she is its first owner).
pub(super) async fn owned(cx: &mut Cx<'_>, label: &str) -> Option<Group> {
    let d = cx.draft(label);
    owned_from(cx, d).await
}

pub(super) async fn owned_from(cx: &mut Cx<'_>, d: GroupDraft) -> Option<Group> {
    let made = cx.g.create(d).await;
    cx.ensure(
        made.is_ok(),
        &format!("setup: create ({:?})", made.as_ref().err()),
    );
    made.ok()
}

/// The owner membership of `p` in `g`, as the asker reads it.
pub(super) async fn owner_of(
    v: &dyn GroupGovernance,
    g: ObjectId,
    p: &PersonRef,
) -> Option<Membership> {
    v.owners(g).await.ok()?.into_iter().find(|m| &m.person == p)
}

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    let Some(v) = cx.gov else { return };
    first_owner(cx, v).await;
    only_owners_decide(cx, v).await;
    last_owner(cx, v).await;
    super::rules_owner_more::rules(cx, v).await;
    super::rules_flow_owner::rules(cx, v).await;
    super::rules_owner_exit::rules(cx, v).await;
    cx.act_me().await;
}

async fn first_owner(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    let Some(g) = owned(cx, "own first").await else {
        return;
    };
    let me = cx.me.clone();
    cx.ensure(
        v.owners(g.id)
            .await
            .is_ok_and(|o| o.len() == 1 && o[0].person == me && o[0].role == Some(role_owner())),
        "create: the creator is the first owner (direct member with the owner role)",
    );
    cx.ensure(
        v.my_level(g.id).await == Ok(Level::Owner),
        "my_level: the creator is Owner",
    );
    cx.ensure(
        cx.g.my_rights(g.id)
            .await
            .is_ok_and(|r| r.contains(&Right::Manage)),
        "create: Own includes Manage (my_rights)",
    );
    let Some(p2) = cx.person(Kind::Human).await else {
        return;
    };
    cx.ensure(v.my_level(g.id).await.is_ok(), "my_level: readable");
    cx.ensure(
        v.appoint_owner(g.id, &p2).await.is_ok(),
        "appoint_owner: an owner appoints a second owner (several owners)",
    );
    cx.ensure(
        v.owners(g.id).await.is_ok_and(|o| o.len() == 2),
        "owners: both are listed",
    );
    if cx.act(&p2).await {
        cx.ensure(
            v.my_level(g.id).await == Ok(Level::Owner),
            "my_level: the appointed one is Owner",
        );
        cx.ensure(
            cx.g.my_rights(g.id)
                .await
                .is_ok_and(|r| r.contains(&Right::Manage)),
            "appoint_owner: the new owner may manage",
        );
        cx.act_me().await;
    }
    let again = v.appoint_owner(g.id, &p2).await;
    cx.ensure(
        matches!(err(&again), Some(Error::AlreadyThere(_))),
        "appoint_owner: twice is AlreadyThere",
    );
}

async fn only_owners_decide(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    let Some(g) = owned(cx, "own gate").await else {
        return;
    };
    let (Some(mgr), Some(stranger), Some(cand)) = (
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
    ) else {
        return;
    };
    let cand_m = cx.member(g.id, &cand).await;
    let adder = cx.person(Kind::Human).await;
    cx.ensure(
        cx.g.grant(g.id, &mgr, Right::Manage).await.is_ok(),
        "setup: a manager by right",
    );
    let me = cx.me.clone();
    let Some(mine) = owner_of(v, g.id, &me).await else {
        return;
    };
    if cx.act(&mgr).await {
        cx.ensure(
            v.my_level(g.id).await == Ok(Level::Manager),
            "my_level: Manage by right is Manager",
        );
        cx.ensure(
            matches!(
                err(&v.appoint_owner(g.id, &cand).await),
                Some(Error::NeedsOwner { .. })
            ),
            "appoint_owner: without the Owner level it is NeedsOwner",
        );
        cx.ensure(
            matches!(
                err(&v.demote_owner(mine.id, mine.revision).await),
                Some(Error::NeedsOwner { .. })
            ),
            "demote_owner: without the Owner level it is NeedsOwner",
        );
        cx.ensure(
            matches!(
                err(&v
                    .transfer_ownership(mine.id, mine.revision, &cand, false)
                    .await),
                Some(Error::NotYourMembership(_))
            ),
            "transfer_ownership: a manager cannot hand over someone else's ownership",
        );
        cx.ensure(
            matches!(
                err(&cx.g.set_role(mine.id, mine.revision, None).await),
                Some(Error::Forbidden { .. })
            ),
            "set_role: a manager cannot take the owner role away (member path)",
        );
        cx.ensure(
            matches!(
                err(&cx.g.remove_member(mine.id, mine.revision).await),
                Some(Error::Forbidden { .. })
            ),
            "remove_member: a manager cannot remove an owner (member path)",
        );
        let rev = cx.rev(g.id).await;
        cx.ensure(
            matches!(
                err(&cx.g.delete(g.id, rev).await),
                Some(Error::Forbidden { .. })
            ),
            "delete: a manager cannot delete a group that has an owner",
        );
        super::rules_owner_roles::v1_role_paths(cx, g.id, &stranger, cand_m.as_ref()).await;
        if let Some(adder) = adder {
            super::rules_owner_roles::adder_cannot_make_managers(cx, g.id, &adder, &stranger).await;
        }
    }
    if cx.act(&stranger).await {
        cx.ensure(
            v.my_level(g.id).await == Ok(Level::None),
            "my_level: a stranger has no level",
        );
        cx.ensure(
            matches!(
                err(&v.appoint_owner(g.id, &stranger).await),
                Some(Error::NeedsOwner { .. })
            ),
            "appoint_owner: a stranger cannot make herself an owner",
        );
        cx.ensure(
            cx.g.update(
                g.id,
                g.revision,
                crate::groups::GroupPatch {
                    name: Some("Zzconf hijack".into()),
                    ..Default::default()
                },
            )
            .await
            .is_err(),
            "update: a foreign group is not written",
        );
        cx.act_me().await;
    }
    cx.ensure(
        cx.fresh(g.id).await.is_some_and(|x| x.name == g.name),
        "the group is unchanged after the refused writes",
    );
}

async fn last_owner(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    let Some(g) = owned(cx, "own last").await else {
        return;
    };
    let me = cx.me.clone();
    let Some(mine) = owner_of(v, g.id, &me).await else {
        return;
    };
    cx.ensure(
        matches!(
            err(&v.demote_owner(mine.id, mine.revision).await),
            Some(Error::LastOwner { .. })
        ),
        "demote_owner: the last owner stays (LastOwner)",
    );
    cx.ensure(
        matches!(
            err(&cx.g.remove_member(mine.id, mine.revision).await),
            Some(Error::LastManager)
        ),
        "remove_member: the last owner cannot leave (the member path says LastManager)",
    );
    cx.ensure(
        matches!(
            err(&cx.g.set_role(mine.id, mine.revision, None).await),
            Some(Error::LastManager)
        ),
        "set_role: the last owner cannot be made a plain member (the member path says LastManager)",
    );
    let Some(p2) = cx.person(Kind::Human).await else {
        return;
    };
    cx.ensure(
        v.appoint_owner(g.id, &p2).await.is_ok(),
        "setup: second owner",
    );
    let Some(mine) = owner_of(v, g.id, &me).await else {
        return;
    };
    cx.ensure(
        v.demote_owner(mine.id, mine.revision)
            .await
            .is_ok_and(|m| m.role.is_none()),
        "demote_owner: with another owner it works and leaves a plain member",
    );
    cx.ensure(
        v.my_level(g.id).await.is_ok_and(|l| l != Level::Owner),
        "my_level: no longer Owner",
    );
    cx.ensure(
        v.owners(g.id).await.is_ok_and(|o| o.len() == 1),
        "owners: one left",
    );
}
