//! Rules of `groups`, addons (part 2): independence (an addon changes nothing else) and self-management.
//!
//! Pattern: Template method (a fixed list of rules; each one is a mutant killer).

use super::cx::Cx;
use super::fixture::Kind;
use super::rules_addon::{formal, other};
use super::rules_owner::{owned, owner_of};
use crate::groups::GroupGovernance;
use crate::groups::{NestingSide, Right};

pub(super) async fn rules(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    independent(cx, v).await;
    self_managed(cx, v).await;
}

async fn independent(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    let (Some(g), Some(p)) = (owned(cx, "add indep").await, cx.person(Kind::Human).await) else {
        return;
    };
    let Some(parent) = cx.group("add indep parent").await else {
        return;
    };
    cx.member(g.id, &p).await;
    cx.link(g.id, parent.id, crate::groups::NestingMode::BottomUp)
        .await;
    let before = snapshot(cx, v, g.id, &p, parent.id).await;
    cx.ensure(
        v.attach_addon(g.id, formal("Zzconf indep")).await.is_ok(),
        "setup: attach",
    );
    cx.ensure(
        v.attach_addon(g.id, other()).await.is_ok(),
        "setup: attach another",
    );
    cx.ensure(
        before == snapshot(cx, v, g.id, &p, parent.id).await,
        "addons are only attached: members, rights, edges and owners are the same with and without",
    );
}

async fn self_managed(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    let (Some(g), Some(p2)) = (owned(cx, "add self").await, cx.person(Kind::Human).await) else {
        return;
    };
    cx.ensure(
        v.appoint_owner(g.id, &p2).await.is_ok(),
        "setup: second owner",
    );
    cx.ensure(
        v.attach_addon(g.id, formal("Zzconf self")).await.is_ok(),
        "setup: attach by the first owner",
    );
    if cx.act(&p2).await {
        let Some(a) = v.addons(g.id).await.ok().and_then(|mut l| l.pop()) else {
            return;
        };
        cx.ensure(
            v.update_addon(a.id, a.revision, formal("Zzconf self 2"))
                .await
                .is_ok(),
            "the group manages itself: the second owner changes the addon",
        );
        cx.ensure(
            v.attach_addon(g.id, other()).await.is_ok(),
            "the group manages itself: ... and attaches another",
        );
        cx.act_me().await;
    }
    let me = cx.me.clone();
    cx.ensure(
        owner_of(v, g.id, &me).await.is_some(),
        "the first owner is still an owner",
    );
}

type Snapshot = (
    Vec<crate::groups::shared::PersonRef>,
    Option<usize>,
    Option<bool>,
    Option<usize>,
    Option<Vec<Right>>,
    Option<usize>,
);

/// What addons must not change: effective members above, members, `may_do`, edges, rights, owners.
async fn snapshot(
    cx: &Cx<'_>,
    v: &dyn GroupGovernance,
    g: crate::groups::shared::ObjectId,
    p: &crate::groups::shared::PersonRef,
    parent: crate::groups::shared::ObjectId,
) -> Snapshot {
    (
        cx.effective(parent)
            .await
            .into_iter()
            .map(|e| e.person)
            .collect(),
        cx.g.members(g, None, 100).await.map(|m| m.items.len()).ok(),
        cx.g.may_do(g, p, Right::Manage)
            .await
            .map(|d| d.allowed)
            .ok(),
        cx.g.nestings(g, NestingSide::AsChild)
            .await
            .map(|n| n.len())
            .ok(),
        cx.g.my_rights(g).await.ok(),
        v.owners(g).await.map(|o| o.len()).ok(),
    )
}
