//! Rules: the mirror plan and run against a directory (Kanidm): dry run, ticket, findings per case, native nesting
//! per mode, the AI and admin rules on the hull.
//!
//! Pattern: Template method (a fixed list of rules against the fixture's `Mirroring` and `GroupDirectory`).

use super::cx::Cx;
use super::fixture::Kind;
use crate::groups::shared::{ObjectId, PersonRef};
use crate::groups::{
    FindingKind, GroupDirectory, GroupMirrorState, MirrorError, MirrorPlan, MirrorStep, Mirroring,
    NestingMode,
};

/// The parts of an `Update` step: add, remove, nest, unnest.
pub(crate) type UpdateParts<'a> = (
    &'a Vec<PersonRef>,
    &'a Vec<PersonRef>,
    &'a Vec<ObjectId>,
    &'a Vec<ObjectId>,
);

pub(crate) fn update_of(plan: &MirrorPlan, g: ObjectId) -> Option<UpdateParts<'_>> {
    plan.steps.iter().find_map(|s| match s {
        MirrorStep::Update {
            group,
            add,
            remove,
            nest,
            unnest,
        } if *group == g => Some((add, remove, nest, unnest)),
        _ => None,
    })
}

pub(crate) fn has(plan: &MirrorPlan, kind: &FindingKind) -> bool {
    plan.findings.iter().any(|f| &f.kind == kind)
}

pub(crate) async fn mirrored(cx: &mut Cx<'_>, label: &str) -> Option<crate::groups::Group> {
    let mut d = cx.draft(label);
    d.mirror = true;
    cx.group_from(d).await
}

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    let (Some(mir), Some(dir)) = (cx.fx.mirroring(), cx.fx.directory()) else {
        cx.skip("mirror: the fixture has no Mirroring or directory");
        return;
    };
    plan_and_run(cx, mir, dir).await;
    super::rules_mirror_more::rules(cx).await;
    super::rules_mirror_trees::rules(cx).await;
}

async fn plan_and_run(cx: &mut Cx<'_>, mir: &dyn Mirroring, dir: &dyn GroupDirectory) {
    let (Some(m), Some(c), Some(t1), Some(t2), Some(s)) = (
        mirrored(cx, "mi m").await,
        mirrored(cx, "mi c").await,
        mirrored(cx, "mi t1").await,
        mirrored(cx, "mi t2").await,
        mirrored(cx, "mi s").await,
    ) else {
        return;
    };
    let (Some(plain), Some(sens), Some(p)) = (
        cx.group("mi plain").await,
        cx.sensitive_group("mi sens").await,
        cx.person(Kind::Human).await,
    ) else {
        return;
    };
    cx.member(m.id, &p).await;
    cx.link(c.id, m.id, NestingMode::BottomUp).await;
    cx.link(t2.id, t1.id, NestingMode::TopDown).await;
    cx.link(s.id, m.id, NestingMode::Single).await;
    let Ok(plan) = mir.mirror_plan(dir).await else {
        cx.ensure(false, "mirror_plan: a dry run");
        return;
    };
    let creates = |g: ObjectId| {
        plan.steps
            .iter()
            .any(|x| matches!(x, MirrorStep::Create { group } if *group == g))
    };
    cx.ensure(
        creates(m.id) && creates(c.id) && !creates(plain.id) && !creates(sens.id),
        "mirror_plan: Create for mirrored groups only, never plain or sensitive groups",
    );
    cx.ensure(
        update_of(&plan, m.id)
            .is_some_and(|u| u.0.contains(&p) && u.2.contains(&c.id) && !u.2.contains(&s.id)),
        "mirror_plan: BottomUp, the child is nested in the parent; Single is not an arc",
    );
    cx.ensure(
        update_of(&plan, t2.id).is_some_and(|u| u.2.contains(&t1.id)),
        "mirror_plan: TopDown, the parent is nested in the child",
    );
    cx.ensure(
        dir.read(m.id).await.is_ok_and(|g| g.is_none()),
        "mirror_plan: writes nothing to the directory",
    );
    let Ok(report) = mir.mirror_apply(dir, plan.ticket.clone(), &[]).await else {
        cx.ensure(false, "mirror_apply: runs the plan that was shown");
        return;
    };
    cx.ensure(
        !report.applied.is_empty() && report.directory == plan.directory,
        "mirror_apply: the report holds the applied steps",
    );
    let held = dir.read(m.id).await.ok().flatten();
    cx.ensure(
        held.as_ref().is_some_and(|g| {
            g.members == vec![p.clone()] && g.nested.contains(&c.id) && g.name == m.name
        }),
        "mirror_apply: the directory holds the persons and the native nesting",
    );
    cx.ensure(
        dir.read(t2.id)
            .await
            .is_ok_and(|g| g.is_some_and(|g| g.nested == vec![t1.id])),
        "mirror_apply: TopDown is native too",
    );
    cx.ensure(
        dir.read(sens.id).await.is_ok_and(|g| g.is_none()),
        "mirror_apply: sensitive groups are not in the directory",
    );
    let again = mir.mirror_plan(dir).await;
    cx.ensure(
        again.is_ok_and(|a| update_of(&a, m.id).is_none() && update_of(&a, c.id).is_none()),
        "mirror_plan: after the run nothing more to do for them (idempotent)",
    );
    let status = mir.mirror_status().await.unwrap_or_default();
    cx.ensure(
        status
            .iter()
            .any(|x| x.directory == report.directory && x.last.is_some()),
        "mirror_status: the last report per directory",
    );
    let states = mir.group_mirror_state(m.id).await.unwrap_or_default();
    cx.ensure(
        states
            .iter()
            .any(|(d, st)| *d == report.directory && matches!(st, GroupMirrorState::Synced { .. })),
        "group_mirror_state: Synced after the run",
    );
    cx.ensure(
        mir.group_mirror_state(plain.id)
            .await
            .is_ok_and(|v| v.iter().all(|(_, st)| *st == GroupMirrorState::NotMirrored)),
        "group_mirror_state: a group that is not mirrored says NotMirrored",
    );
    // A group that is not mirrored while its child is: the arc is missing in the directory, said out loud.
    let (Some(outer), Some(inner)) = (mirrored(cx, "mi outer").await, cx.group("mi inner").await)
    else {
        return;
    };
    cx.link(inner.id, outer.id, NestingMode::BottomUp).await;
    let plan = mir.mirror_plan(dir).await;
    cx.ensure(
        plan.is_ok_and(|p| {
            has(
                &p,
                &FindingKind::NestedNotMirrored {
                    outer: outer.id,
                    inner: inner.id,
                },
            )
        }),
        "mirror_plan: a nesting to a group that is not mirrored is NestedNotMirrored",
    );
    // A stale ticket.
    let Ok(old) = mir.mirror_plan(dir).await else {
        return;
    };
    cx.group("mi later").await;
    let mut d = cx.draft("mi later2");
    d.mirror = true;
    cx.group_from(d).await;
    let r = mir.mirror_apply(dir, old.ticket, &[]).await;
    cx.ensure(
        matches!(r, Err(MirrorError::PlanStale)),
        "mirror_apply: a plan that is not the current one is PlanStale",
    );
    // A group deleted after the run: Remove, and its arc is an unnest.
    let rev = cx.rev(c.id).await;
    cx.g.delete(c.id, rev).await.ok();
    let plan = mir.mirror_plan(dir).await;
    cx.ensure(
        plan.as_ref().is_ok_and(|p| {
            p.steps
                .iter()
                .any(|x| matches!(x, MirrorStep::Remove { group } if *group == c.id))
                && update_of(p, m.id).is_some_and(|u| u.3.contains(&c.id))
        }),
        "mirror_plan: a deleted group is a Remove step, the arc an unnest",
    );
}
