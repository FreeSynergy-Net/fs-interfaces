//! Rules of `groups_v2`, scopes and AI (ADR 0095 Pkt. 13): a scope never works for an AI account or an account of
//! unknown kind, neither to open an intervention nor to hand out scopes.
//!
//! Pattern: Template method (each rule is a mutant killer: drop the human check of the scope and a check fails).

use super::cx::Cx;
use super::fixture::Kind;
use super::rules_owner::owned;
use crate::groups_v1::shared::PersonRef;
use crate::groups_v2::{Error2, GroupsV2, Scope};

fn denied<T>(r: &Result<T, Error2>) -> bool {
    matches!(r, Err(Error2::ScopeDenied(_)))
}

/// `holder` holds the `Project` scope and is the asker when this runs.
pub(crate) async fn rules(cx: &mut Cx<'_>, v: &dyn GroupsV2, holder: &PersonRef) {
    cx.act_me().await;
    let (Some(q), Some(target), Some(admin_c), Some(admin_d)) = (
        cx.person(Kind::Human).await,
        owned(cx, v, "sc ai target").await,
        cx.group("sc ai admin c").await,
        cx.group("sc ai admin d").await,
    ) else {
        return;
    };
    cx.member(admin_c.id, &q).await;
    cx.act(holder).await;
    cx.ensure(
        v.grant_scope(admin_c.id, Scope::AllGroups).await.is_ok(),
        "setup: a scope for a group with a later AI member",
    );
    for (kind, what) in [
        (Kind::Agent, "an AI account"),
        (Kind::Unknown, "an unknown account"),
    ] {
        cx.act_me().await;
        cx.ensure(cx.fx.mark(&q, kind).await, "setup: the mark changes");
        cx.act(&q).await;
        cx.ensure(
            denied(
                &v.open_intervention(target.id, Scope::AllGroups, "Zzconf ki")
                    .await,
            ),
            &format!("open_intervention: a scope does not work for {what}"),
        );
    }
    cx.act_me().await;
    cx.fx.mark(&q, Kind::Human).await;
    cx.ensure(
        cx.fx.mark(holder, Kind::Agent).await,
        "setup: the holder becomes an AI mark",
    );
    cx.act(holder).await;
    cx.ensure(
        denied(&v.grant_scope(admin_d.id, Scope::AllGroups).await),
        "grant_scope: the Project scope does not work for an AI account",
    );
    cx.act_me().await;
    cx.fx.mark(holder, Kind::Human).await;
}
