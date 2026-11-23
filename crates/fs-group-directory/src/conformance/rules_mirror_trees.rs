//! Rules of `groups`: two trees of UNEQUAL depth, one built bottom-up and one top-down, give the same kind of
//! Kanidm tree. Kanidm knows one direction only ("the group X is a member of Y", rights flow to Y): a `BottomUp`
//! edge is mirrored as it is, a `TopDown` edge REVERSED, a `Single` edge not at all (ADR 0082:102-106).
//!
//! Pattern: Template method (the same check function for both trees; each row is a mutant killer: a provider that
//! does not reverse a `TopDown` edge, or mirrors a `Single` one, fails its row).

use super::cx::Cx;
use super::rules_mirror::mirrored;
use crate::groups::shared::ObjectId;
use crate::groups::{GroupDirectory, NestingMode};

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    let (Some(mir), Some(dir)) = (cx.fx.mirroring(), cx.fx.directory()) else {
        return;
    };
    let (Some(a_top), Some(a_mid), Some(a_low1), Some(a_low2), Some(a_adv)) = (
        mirrored(cx, "tree up top").await,
        mirrored(cx, "tree up mid").await,
        mirrored(cx, "tree up low 1").await,
        mirrored(cx, "tree up low 2").await,
        mirrored(cx, "tree up advisory").await,
    ) else {
        return;
    };
    let (Some(b_top), Some(b_dept1), Some(b_dept2), Some(b_team)) = (
        mirrored(cx, "tree down top").await,
        mirrored(cx, "tree down dept 1").await,
        mirrored(cx, "tree down dept 2").await,
        mirrored(cx, "tree down team").await,
    ) else {
        return;
    };
    // Bottom-up, depth three on one branch and two on the other; one advisory (Single) edge.
    cx.link(a_low1.id, a_mid.id, NestingMode::BottomUp).await;
    cx.link(a_mid.id, a_top.id, NestingMode::BottomUp).await;
    cx.link(a_low2.id, a_top.id, NestingMode::BottomUp).await;
    cx.link(a_adv.id, a_mid.id, NestingMode::Single).await;
    // Top-down, depth three on one branch and two on the other.
    cx.link(b_dept1.id, b_top.id, NestingMode::TopDown).await;
    cx.link(b_dept2.id, b_top.id, NestingMode::TopDown).await;
    cx.link(b_team.id, b_dept1.id, NestingMode::TopDown).await;
    let Ok(plan) = mir.mirror_plan(dir).await else {
        cx.ensure(false, "mirror trees: a dry run");
        return;
    };
    cx.ensure(
        mir.mirror_apply(dir, plan.ticket.clone(), &[])
            .await
            .is_ok(),
        "mirror trees: the plan is applied",
    );
    // Bottom-up: the directory group of the upper holds the lower as member.
    nested_is(cx, dir, "bottom-up top", a_top.id, &[a_mid.id, a_low2.id]).await;
    nested_is(
        cx,
        dir,
        "bottom-up mid (the single edge is not mirrored)",
        a_mid.id,
        &[a_low1.id],
    )
    .await;
    nested_is(cx, dir, "bottom-up leaf", a_low1.id, &[]).await;
    nested_is(cx, dir, "advisory group", a_adv.id, &[]).await;
    // Top-down: REVERSED, the directory group of the lower holds the upper as member.
    nested_is(cx, dir, "top-down team", b_team.id, &[b_dept1.id]).await;
    nested_is(cx, dir, "top-down dept 1", b_dept1.id, &[b_top.id]).await;
    nested_is(cx, dir, "top-down dept 2", b_dept2.id, &[b_top.id]).await;
    nested_is(cx, dir, "top-down top", b_top.id, &[]).await;
}

/// The directory group `g` holds exactly `want` as nested groups.
async fn nested_is(
    cx: &mut Cx<'_>,
    dir: &dyn GroupDirectory,
    what: &str,
    g: ObjectId,
    want: &[ObjectId],
) {
    let held = dir.read(g).await.ok().flatten();
    let mut got: Vec<ObjectId> = held.map(|d| d.nested).unwrap_or_default();
    let mut want = want.to_vec();
    got.sort();
    want.sort();
    cx.ensure(
        got == want,
        &format!("mirror trees: {what}: the directory tree is the member tree (top-down reversed, single not mirrored)"),
    );
}
