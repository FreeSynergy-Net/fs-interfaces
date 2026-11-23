//! Rules: depth, sensitivity, mode changes, `nesting_effect` and walking the tree.
//!
//! Pattern: Template method (a fixed list of rules; each rule seeds its own data through the trait).

use super::cx::{Cx, err};
use super::fixture::Kind;
use crate::groups_v1::{
    ConflictKind, Error, GroupPatch, MAX_NESTING_DEPTH, NestOutcome, NestingMode, NestingSide,
    Right,
};

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    depth(cx).await;
    sensitive(cx).await;
    mode_changes(cx).await;
    effect(cx).await;
    walks(cx).await;
}

async fn depth(cx: &mut Cx<'_>) {
    let mut chain = Vec::new();
    for i in 0..=MAX_NESTING_DEPTH + 1 {
        let Some(g) = cx.group(&format!("dp{i}")).await else {
            return;
        };
        chain.push(g);
    }
    for w in chain.windows(2).take(MAX_NESTING_DEPTH as usize) {
        cx.link(w[0].id, w[1].id, NestingMode::BottomUp).await;
    }
    let n = chain.len();
    let r =
        cx.g.request_nesting(chain[n - 2].id, chain[n - 1].id, NestingMode::BottomUp)
            .await;
    cx.ensure(
        matches!(err(&r), Some(Error::TooDeep { max }) if *max == MAX_NESTING_DEPTH),
        "depth: a chain of more than 16 edges is TooDeep",
    );
}

async fn sensitive(cx: &mut Cx<'_>) {
    let (Some(s), Some(p)) = (
        cx.sensitive_group("sens").await,
        cx.group("sens plain").await,
    ) else {
        return;
    };
    for (child, parent) in [(s.id, p.id), (p.id, s.id)] {
        let r =
            cx.g.request_nesting(child, parent, NestingMode::BottomUp)
                .await;
        cx.ensure(
            matches!(
                err(&r),
                Some(Error::ModeConflict(ConflictKind::SensitiveFlow { .. }))
            ),
            "sensitive: no arc between a sensitive and a plain group, either way",
        );
    }
    cx.ensure(
        matches!(
            cx.g.request_nesting(s.id, p.id, NestingMode::Single).await,
            Ok(NestOutcome::Linked(_))
        ),
        "sensitive: Single stays allowed",
    );
    let (Some(a), Some(b)) = (cx.group("sens a").await, cx.group("sens b").await) else {
        return;
    };
    cx.link(a.id, b.id, NestingMode::BottomUp).await;
    let rev = cx.rev(a.id).await;
    let r =
        cx.g.update(
            a.id,
            rev,
            GroupPatch {
                sensitive: Some(true),
                ..GroupPatch::default()
            },
        )
        .await;
    cx.ensure(
        matches!(err(&r), Some(Error::ModeConflict(_))),
        "sensitive: update(sensitive) on a group in an arc is ModeConflict",
    );
}

async fn mode_changes(cx: &mut Cx<'_>) {
    let (Some(a), Some(b)) = (cx.group("mc a").await, cx.group("mc b").await) else {
        return;
    };
    let Some(x) = cx.person(Kind::Human).await else {
        return;
    };
    cx.member(a.id, &x).await;
    let Some(e) = cx.link(a.id, b.id, NestingMode::BottomUp).await else {
        return;
    };
    cx.ensure(cx.is_effective(b.id, &x).await, "mode: BottomUp flows");
    let same =
        cx.g.change_nesting_mode(e.id, e.revision, NestingMode::BottomUp)
            .await;
    cx.ensure(
        matches!(err(&same), Some(Error::AlreadyThere(id)) if *id == e.id),
        "mode: the same mode is AlreadyThere",
    );
    let stale =
        cx.g.change_nesting_mode(
            e.id,
            crate::groups_v1::shared::Revision(e.revision.0 + 9),
            NestingMode::Single,
        )
        .await;
    cx.ensure(
        matches!(err(&stale), Some(Error::Changed { .. })),
        "mode: an old revision is Changed",
    );
    let Ok(NestOutcome::Linked(weak)) =
        cx.g.change_nesting_mode(e.id, e.revision, NestingMode::Single)
            .await
    else {
        cx.ensure(false, "mode: weakening to Single applies at once");
        return;
    };
    cx.ensure(
        weak.mode == NestingMode::Single
            && weak.revision > e.revision
            && !cx.is_effective(b.id, &x).await,
        "mode: Single, the revision rises, the flow stops",
    );
    let Ok(NestOutcome::Linked(down)) =
        cx.g.change_nesting_mode(e.id, weak.revision, NestingMode::TopDown)
            .await
    else {
        cx.ensure(
            false,
            "mode: both sides managed, a stronger mode applies at once",
        );
        return;
    };
    cx.ensure(down.mode == NestingMode::TopDown, "mode: TopDown");
    let ns =
        cx.g.nestings(a.id, NestingSide::AsChild)
            .await
            .unwrap_or_default();
    cx.ensure(
        ns.len() == 1 && ns[0].mode == NestingMode::TopDown,
        "nestings: the edge shows its mode",
    );
    cx.ensure(
        cx.g.remove_nesting(e.id, e.revision).await.is_err(),
        "remove_nesting: an old revision fails",
    );
    cx.ensure(
        cx.g.remove_nesting(e.id, down.revision).await.is_ok(),
        "remove_nesting: one side's Manage dissolves",
    );
    cx.ensure(
        cx.g.nestings(a.id, NestingSide::AsChild)
            .await
            .is_ok_and(|e| e.is_empty()),
        "remove_nesting: the edge is gone",
    );
}

async fn effect(cx: &mut Cx<'_>) {
    let (Some(c), Some(p)) = (cx.group("fx child").await, cx.group("fx parent").await) else {
        return;
    };
    let (Some(x), Some(y)) = (cx.person(Kind::Human).await, cx.person(Kind::Human).await) else {
        return;
    };
    cx.member(c.id, &x).await;
    cx.member(p.id, &y).await;
    let r = cx.g.nesting_effect(c.id, p.id, NestingMode::BottomUp).await;
    cx.ensure(
        r.as_ref().is_ok_and(|e| {
            e.groups_affected == 1
                && e.persons_gained == 1
                && e.persons_lost == 0
                && e.mode == NestingMode::BottomUp
        }),
        "nesting_effect: BottomUp gains the child's person in the parent",
    );
    let r = cx.g.nesting_effect(c.id, p.id, NestingMode::Single).await;
    cx.ensure(
        r.is_ok_and(|e| e.groups_affected == 0),
        "nesting_effect: Single changes nobody",
    );
    cx.ensure(
        matches!(
            err(&cx.g.nesting_effect(c.id, c.id, NestingMode::BottomUp).await),
            Some(Error::Cycle { .. })
        ),
        "nesting_effect: the same checks, a Cycle",
    );
    let before =
        cx.g.nestings(c.id, NestingSide::AsChild)
            .await
            .map_or(9, |e| e.len());
    cx.ensure(before == 0, "nesting_effect: writes nothing");
    if cx.link(c.id, p.id, NestingMode::BottomUp).await.is_some() {
        let r = cx.g.nesting_effect(c.id, p.id, NestingMode::TopDown).await;
        cx.ensure(
            r.is_ok_and(|e| e.persons_lost == 1 && e.persons_gained == 1),
            "nesting_effect: a mode change loses and gains",
        );
    }
    if let Some(other) = cx.person(Kind::Human).await {
        if cx.act(&other).await {
            cx.ensure(
                matches!(
                    err(&cx.g.nesting_effect(c.id, p.id, NestingMode::TopDown).await),
                    Some(Error::Forbidden {
                        right: Right::Manage,
                        ..
                    })
                ),
                "nesting_effect: without Manage on a side it is Forbidden",
            );
            cx.act_me().await;
        }
    }
}

async fn walks(cx: &mut Cx<'_>) {
    let (Some(a), Some(b), Some(c), Some(d)) = (
        cx.group("wk a").await,
        cx.group("wk b").await,
        cx.group("wk c").await,
        cx.group("wk d").await,
    ) else {
        return;
    };
    cx.link(a.id, b.id, NestingMode::Single).await;
    cx.link(a.id, c.id, NestingMode::BottomUp).await;
    cx.link(b.id, d.id, NestingMode::TopDown).await;
    cx.link(c.id, d.id, NestingMode::BottomUp).await;
    let up = cx.g.ancestors(a.id).await.unwrap_or_default();
    cx.ensure(
        up.len() == 3 && up.contains(&d.id) && up.iter().filter(|g| **g == d.id).count() == 1,
        "ancestors: all modes, a diamond counts each group once",
    );
    cx.ensure(
        cx.g.descendants(d.id)
            .await
            .is_ok_and(|v| v.len() == 3 && v.contains(&a.id)),
        "descendants: all modes",
    );
    cx.ensure(
        cx.g.ancestors(d.id).await.is_ok_and(|v| v.is_empty()),
        "ancestors: a root has none",
    );
    cx.ensure(
        cx.g.nestings(d.id, NestingSide::AsParent)
            .await
            .is_ok_and(|v| v.len() == 2),
        "nestings: the parent side lists its children",
    );
    let Some(x) = cx.person(Kind::Human).await else {
        return;
    };
    cx.member(a.id, &x).await;
    let eff = cx.effective(d.id).await;
    cx.ensure(
        eff.iter().filter(|m| m.person == x).count() == 1,
        "diamond: a person counts once per group",
    );
}
