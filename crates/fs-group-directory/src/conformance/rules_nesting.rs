//! Rules: the three modes, cycles, depth, sensitivity, mode changes and `nesting_effect`.
//!
//! Pattern: Template method (a fixed list of rules; the people and groups are seeded through the trait).

use super::cx::{Cx, err};
use super::fixture::Kind;
use crate::groups_v1::{Error, NestingMode, NestingSide, Reason, Right};

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    flows(cx).await;
    rights_flow(cx).await;
    unequal_depth(cx).await;
    cycles(cx).await;
    super::rules_nesting_more::rules(cx).await;
}

/// Who counts where, per mode (`nesting.rs`).
async fn flows(cx: &mut Cx<'_>) {
    let (Some(up), Some(low)) = (cx.group("bu up").await, cx.group("bu low").await) else {
        return;
    };
    let (Some(x), Some(y)) = (cx.person(Kind::Human).await, cx.person(Kind::Human).await) else {
        return;
    };
    cx.member(low.id, &x).await;
    cx.member(up.id, &y).await;
    cx.link(low.id, up.id, NestingMode::BottomUp).await;
    let eff = cx.effective(up.id).await;
    let got = eff.iter().find(|m| m.person == x);
    cx.ensure(
        got.is_some_and(|m| {
            m.direct.is_none()
                && m.inherited
                    .iter()
                    .any(|i| i.from == Some(low.id) && i.via == vec![low.id, up.id])
        }),
        "BottomUp: members count upwards, inherited from the child with the way",
    );
    cx.ensure(
        eff.iter()
            .any(|m| m.person == y && m.direct.is_some() && m.inherited.is_empty()),
        "BottomUp: a direct member is direct",
    );
    cx.ensure(
        !cx.is_effective(low.id, &y).await,
        "BottomUp: nothing flows down",
    );
    cx.ensure(
        cx.g.groups_of(&x).await.is_ok_and(|g| {
            g.iter()
                .any(|e| e.group == up.id && e.direct.is_none() && !e.inherited.is_empty())
        }),
        "groups_of: a group over an edge is listed with its origin",
    );

    let (Some(chiefs), Some(deputies)) =
        (cx.group("td chiefs").await, cx.group("td deputies").await)
    else {
        return;
    };
    let (Some(c), Some(d)) = (cx.person(Kind::Human).await, cx.person(Kind::Human).await) else {
        return;
    };
    cx.member(chiefs.id, &c).await;
    cx.member(deputies.id, &d).await;
    cx.link(deputies.id, chiefs.id, NestingMode::TopDown).await;
    cx.ensure(
        cx.is_effective(deputies.id, &c).await,
        "TopDown: the upper group is a member of the lower (Chiefs are also Deputies)",
    );
    cx.ensure(
        !cx.is_effective(chiefs.id, &d).await,
        "TopDown: the lower group's members do not count above",
    );

    let (Some(a), Some(b)) = (cx.group("single a").await, cx.group("single b").await) else {
        return;
    };
    let Some(s) = cx.person(Kind::Human).await else {
        return;
    };
    cx.member(b.id, &s).await;
    cx.link(b.id, a.id, NestingMode::Single).await;
    cx.ensure(
        !cx.is_effective(a.id, &s).await
            && cx
                .g
                .nestings(b.id, NestingSide::AsChild)
                .await
                .is_ok_and(|e| e.len() == 1),
        "Single: an edge in the tree, nothing flows",
    );
}

/// Rights follow the member graph: a grant to a group holds for its effective members.
async fn rights_flow(cx: &mut Cx<'_>) {
    let (Some(target), Some(top), Some(under), Some(side)) = (
        cx.group("rf target").await,
        cx.group("rf top").await,
        cx.group("rf under").await,
        cx.group("rf side").await,
    ) else {
        return;
    };
    let (Some(u), Some(t), Some(s)) = (
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
    ) else {
        return;
    };
    cx.member(under.id, &u).await;
    cx.member(top.id, &t).await;
    cx.member(side.id, &s).await;
    cx.link(under.id, top.id, NestingMode::BottomUp).await;
    cx.link(side.id, top.id, NestingMode::Single).await;
    let holder = crate::ids::group_ref(top.id);
    cx.ensure(
        cx.g.grant(target.id, &holder, Right::AddMembers)
            .await
            .is_ok(),
        "grant: a group may hold a right",
    );
    let d = cx.g.may_do(target.id, &u, Right::AddMembers).await;
    cx.ensure(
        d.as_ref().is_ok_and(|d| {
            d.allowed && matches!(d.reason, Reason::ViaGroup { holder: Some(h), .. } if h == top.id)
        }),
        "rights: BottomUp, the upper group's right holds for everyone below",
    );
    cx.ensure(
        cx.g.may_do(target.id, &t, Right::AddMembers)
            .await
            .is_ok_and(|d| d.allowed),
        "rights: a direct member of the holder group may",
    );
    cx.ensure(
        cx.g.may_do(target.id, &s, Right::AddMembers)
            .await
            .is_ok_and(|d| !d.allowed && d.reason == Reason::NoGrant),
        "rights: Single carries no right",
    );
    cx.ensure(
        cx.g.may_do(target.id, &u, Right::Manage)
            .await
            .is_ok_and(|d| !d.allowed),
        "rights: AddMembers is not Manage",
    );
    let (Some(chiefs), Some(deputies), Some(thing)) = (
        cx.group("rf chiefs").await,
        cx.group("rf deputies").await,
        cx.group("rf thing").await,
    ) else {
        return;
    };
    let Some(chief) = cx.person(Kind::Human).await else {
        return;
    };
    cx.member(chiefs.id, &chief).await;
    cx.link(deputies.id, chiefs.id, NestingMode::TopDown).await;
    cx.g.grant(
        thing.id,
        &crate::ids::group_ref(deputies.id),
        Right::RemoveMembers,
    )
    .await
    .ok();
    cx.ensure(
        cx.g.may_do(thing.id, &chief, Right::RemoveMembers)
            .await
            .is_ok_and(|d| d.allowed),
        "rights: TopDown, Chiefs may what the Deputies may",
    );
}

/// Chiefs, Deputies and Team leads: unequal depth, the shortest way, right flows upwards by membership.
async fn unequal_depth(cx: &mut Cx<'_>) {
    let mut chain = Vec::new();
    for i in 0..4 {
        let Some(g) = cx.group(&format!("ud{i}")).await else {
            return;
        };
        chain.push(g);
    }
    let (Some(top), Some(bottom)) = (cx.person(Kind::Human).await, cx.person(Kind::Human).await)
    else {
        return;
    };
    cx.member(chain[0].id, &top).await;
    cx.member(chain[3].id, &bottom).await;
    for w in chain.windows(2) {
        cx.link(w[1].id, w[0].id, NestingMode::TopDown).await;
    }
    let eff = cx.effective(chain[3].id).await;
    let want: Vec<_> = chain.iter().map(|g| g.id).collect();
    cx.ensure(eff.iter().any(|m| m.person == top && m.inherited.iter().any(|i| i.via == want)), "TopDown chain of unequal depth: the top person is a member at the bottom, the way is the chain");
    cx.ensure(
        !cx.is_effective(chain[0].id, &bottom).await,
        "TopDown chain: the bottom person is not a member at the top",
    );
    cx.link(chain[3].id, chain[0].id, NestingMode::Single).await;
    cx.ensure(
        cx.effective(chain[3].id)
            .await
            .iter()
            .any(|m| m.person == top && m.inherited.iter().any(|i| i.via == want)),
        "a Single shortcut does not change the way",
    );
}

async fn cycles(cx: &mut Cx<'_>) {
    let (Some(a), Some(b), Some(c)) = (
        cx.group("cy a").await,
        cx.group("cy b").await,
        cx.group("cy c").await,
    ) else {
        return;
    };
    for mode in [
        NestingMode::BottomUp,
        NestingMode::TopDown,
        NestingMode::Single,
    ] {
        let r = cx.g.request_nesting(a.id, a.id, mode).await;
        cx.ensure(
            matches!(err(&r), Some(Error::Cycle { path }) if *path == vec![a.id, a.id]),
            "cycle: an edge to itself is Cycle [A, A] in every mode",
        );
    }
    let Some(ab) = cx.link(a.id, b.id, NestingMode::BottomUp).await else {
        return;
    };
    let again =
        cx.g.request_nesting(a.id, b.id, NestingMode::BottomUp)
            .await;
    cx.ensure(
        matches!(err(&again), Some(Error::AlreadyThere(id)) if *id == ab.id),
        "edge: the same pair again is AlreadyThere with the id",
    );
    let other = cx.g.request_nesting(a.id, b.id, NestingMode::TopDown).await;
    cx.ensure(
        matches!(err(&other), Some(Error::AlreadyThere(id)) if *id == ab.id),
        "edge: the same pair in another mode is AlreadyThere too",
    );
    let back = cx.g.request_nesting(b.id, a.id, NestingMode::Single).await;
    cx.ensure(
        matches!(err(&back), Some(Error::Cycle { .. })),
        "cycle: the reverse pair is a Cycle in the tree, also with Single",
    );
    cx.link(b.id, c.id, NestingMode::BottomUp).await;
    let mixed = cx.g.request_nesting(a.id, c.id, NestingMode::TopDown).await;
    cx.ensure(matches!(err(&mixed), Some(Error::Cycle { path }) if path.len() >= 3 && path.first() == path.last()), "cycle: a cycle over mixed modes without a tree cycle has a path");
    cx.ensure(
        cx.g.request_nesting(c.id, a.id, NestingMode::BottomUp)
            .await
            .is_err(),
        "cycle: a longer cycle in the tree",
    );
}
