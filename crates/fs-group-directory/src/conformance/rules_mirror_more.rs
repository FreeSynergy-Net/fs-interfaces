//! Rules: by-hand changes in the directory and the AI and admin rules on the hull.
//!
//! Pattern: Template method (a fixed list of rules; each rule seeds its own data through the trait).

use super::cx::Cx;
use super::fixture::{ByHand, Kind};
use super::rules_mirror::{has, mirrored, update_of};
use crate::groups_v1::{FindingKind, GroupDirectory, MirrorError, Mirroring, NestingMode};

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    let (Some(mir), Some(dir)) = (cx.fx.mirroring(), cx.fx.directory()) else {
        return;
    };
    by_hand(cx, mir, dir).await;
    hull_rules(cx, mir, dir).await;
}

async fn by_hand(cx: &mut Cx<'_>, mir: &dyn Mirroring, dir: &dyn GroupDirectory) {
    let (Some(g), Some(other)) = (mirrored(cx, "bh g").await, cx.group("bh other").await) else {
        return;
    };
    let Ok(first) = mir.mirror_plan(dir).await else {
        return;
    };
    if mir.mirror_apply(dir, first.ticket, &[]).await.is_err() {
        return;
    }
    let ok1 = cx
        .fx
        .by_hand(ByHand::Foreign {
            group: g.id,
            account: "hand@example.org".to_owned(),
        })
        .await;
    let ok2 = cx
        .fx
        .by_hand(ByHand::Nest {
            outer: g.id,
            inner: other.id,
        })
        .await;
    if !ok1 || !ok2 {
        cx.skip("mirror: by-hand changes (the fixture cannot)");
        return;
    }
    let Ok(plan) = mir.mirror_plan(dir).await else {
        return;
    };
    let foreign = plan
        .findings
        .iter()
        .find(|f| matches!(&f.kind, FindingKind::Foreign(a) if a.0 == "hand@example.org"));
    let nest = plan.findings.iter().find(|f| {
        f.kind
            == FindingKind::ForeignNesting {
                outer: g.id,
                inner: other.id,
            }
    });
    cx.ensure(foreign.is_some_and(|f| f.overwritable && f.group == Some(g.id)) && nest.is_some_and(|f| f.overwritable), "mirror_plan: by-hand things are findings (Foreign, ForeignNesting), overwritable, never a silent step");
    cx.ensure(
        update_of(&plan, g.id).is_none_or(|u| u.3.is_empty()),
        "mirror_plan: no silent unnest of a nesting by hand",
    );
    let Ok(_) = mir.mirror_apply(dir, plan.ticket.clone(), &[]).await else {
        return;
    };
    let held = dir.read(g.id).await.ok().flatten();
    cx.ensure(
        held.is_some_and(|d| d.foreign.len() == 1 && d.nested.contains(&other.id)),
        "mirror_apply: without a confirmation nothing by hand is removed",
    );
    let Ok(plan) = mir.mirror_plan(dir).await else {
        return;
    };
    let ids: Vec<_> = plan
        .findings
        .iter()
        .filter(|f| f.overwritable && f.group == Some(g.id))
        .map(|f| f.id)
        .collect();
    let Ok(_) = mir.mirror_apply(dir, plan.ticket, &ids).await else {
        return;
    };
    let held = dir.read(g.id).await.ok().flatten();
    cx.ensure(
        held.is_some_and(|d| d.foreign.is_empty() && !d.nested.contains(&other.id)),
        "mirror_apply: confirmed findings are overwritten",
    );
    if cx.fx.by_hand(ByHand::Down(true)).await {
        let r = mir.mirror_plan(dir).await;
        cx.ensure(
            matches!(r, Err(MirrorError::Directory(_))),
            "mirror_plan: a directory that is down is MirrorError::Directory",
        );
        cx.fx.by_hand(ByHand::Down(false)).await;
    }
}

/// The AI rule and the admin rule on the hull: nobody unknown (or AI) becomes an effective member of an admin group.
async fn hull_rules(cx: &mut Cx<'_>, mir: &dyn Mirroring, dir: &dyn GroupDirectory) {
    let Some((_, admin_role)) = cx.fx.roles().await else {
        cx.skip("mirror: the admin rule (the fixture has no roles)");
        return;
    };
    let (Some(admins), Some(low), Some(boss), Some(odd), Some(p)) = (
        mirrored(cx, "hl admins").await,
        mirrored(cx, "hl low").await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Unknown).await,
        cx.person(Kind::Human).await,
    ) else {
        return;
    };
    cx.g.add_member(admins.id, &boss, Some(&admin_role))
        .await
        .ok();
    cx.member(low.id, &odd).await;
    cx.member(low.id, &p).await;
    cx.link(low.id, admins.id, NestingMode::BottomUp).await;
    let Ok(plan) = mir.mirror_plan(dir).await else {
        return;
    };
    cx.ensure(has(&plan, &FindingKind::LeftOut(odd.clone())) && has(&plan, &FindingKind::HeldBack(low.id)), "mirror_plan: an unknown person who would reach an admin group is LeftOut and the arc HeldBack");
    cx.ensure(
        update_of(&plan, admins.id).is_none_or(|u| !u.2.contains(&low.id)),
        "mirror_plan: the arc into the admin group is not written",
    );
    // A person marked as AI after the fact stays out of the directory, and is removed where it already is.
    let Some(g) = mirrored(cx, "hl g").await else {
        return;
    };
    let Some(a) = cx.person(Kind::Human).await else {
        return;
    };
    cx.member(g.id, &a).await;
    let Ok(first) = mir.mirror_plan(dir).await else {
        return;
    };
    if mir.mirror_apply(dir, first.ticket, &[]).await.is_err() || !cx.fx.mark(&a, Kind::Agent).await
    {
        cx.skip("mirror: the AI mark after the fact (the fixture cannot)");
        return;
    }
    let Ok(plan) = mir.mirror_plan(dir).await else {
        cx.ensure(
            false,
            "mirror_plan: does not fail as a whole when an AI sits in a human group",
        );
        return;
    };
    cx.ensure(
        has(&plan, &FindingKind::LeftOut(a.clone()))
            && has(&plan, &FindingKind::HeldBack(g.id))
            && update_of(&plan, g.id).is_some_and(|u| u.1.contains(&a)),
        "mirror_plan: an AI in a human group is LeftOut + HeldBack and removed from the directory",
    );
}
