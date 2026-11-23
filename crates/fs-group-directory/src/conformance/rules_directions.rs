//! Rules of `groups`: what flows over an edge, in BOTH directions and over a `Single` edge (`nesting.rs`). A chain of
//! three groups (low, mid, top) is joined in each mode; a right granted to a group as HOLDER reaches the members of
//! that group, and members of a group below (bottom-up) or above (top-down) count as members of it.
//!
//! Pattern: Template method (a fixed table of expectations per mode; each row is a mutant killer: swap the direction
//! of an edge in a provider and the row of the other mode fails).

use super::cx::Cx;
use super::fixture::Kind;
use crate::groups::shared::PersonRef;
use crate::groups::{Group, NestingMode, Right};
use crate::ids::group_ref;

/// What a mode lets through: (a right of the TOP group reaches the member of LOW, a right of LOW reaches the member
/// of TOP).
const fn flow(mode: NestingMode) -> (bool, bool) {
    match mode {
        NestingMode::BottomUp => (true, false),
        NestingMode::TopDown => (false, true),
        NestingMode::Single => (false, false),
    }
}

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    for (label, mode) in [
        ("bottom-up", NestingMode::BottomUp),
        ("top-down", NestingMode::TopDown),
        ("single", NestingMode::Single),
    ] {
        chain(cx, label, mode).await;
    }
}

/// Whether `who` holds `Read` on `res` (the answer of `may_do`).
async fn reads(cx: &Cx<'_>, res: &Group, who: &PersonRef) -> bool {
    cx.g.may_do(res.id, who, Right::Read)
        .await
        .is_ok_and(|d| d.allowed)
}

async fn chain(cx: &mut Cx<'_>, label: &str, mode: NestingMode) {
    let (Some(low), Some(mid), Some(top), Some(res)) = (
        cx.group(&format!("{label} low")).await,
        cx.group(&format!("{label} mid")).await,
        cx.group(&format!("{label} top")).await,
        cx.group(&format!("{label} resource")).await,
    ) else {
        return;
    };
    let (Some(m_low), Some(m_top)) = (cx.person(Kind::Human).await, cx.person(Kind::Human).await)
    else {
        return;
    };
    cx.member(low.id, &m_low).await;
    cx.member(top.id, &m_top).await;
    cx.link(low.id, mid.id, mode).await;
    cx.link(mid.id, top.id, mode).await;
    let (down, up) = flow(mode);

    // A right of the top group (held as holder) and who of the lower group gets it.
    cx.ensure(
        cx.g.grant(res.id, &group_ref(top.id), Right::Read)
            .await
            .is_ok(),
        "setup: a right held by the top group",
    );
    cx.ensure(
        reads(cx, &res, &m_top).await,
        &format!("{label}: the member of the top group holds the right of the top group"),
    );
    cx.ensure(
        reads(cx, &res, &m_low).await == down,
        &format!("{label}: a right of the top group reaches the member two levels below: {down}"),
    );

    // A right of the low group and who of the upper group gets it (a second resource, so the answers do not mix).
    let Some(res2) = cx.group(&format!("{label} resource 2")).await else {
        return;
    };
    cx.ensure(
        cx.g.grant(res2.id, &group_ref(low.id), Right::Read)
            .await
            .is_ok(),
        "setup: a right held by the low group",
    );
    cx.ensure(
        reads(cx, &res2, &m_low).await,
        &format!("{label}: the member of the low group holds the right of the low group"),
    );
    cx.ensure(
        reads(cx, &res2, &m_top).await == up,
        &format!("{label}: a right of the low group reaches the member two levels above: {up}"),
    );
    let seen_in_mid = cx.is_effective(mid.id, &m_low).await;
    let top_seen_in_mid = cx.is_effective(mid.id, &m_top).await;
    cx.ensure(
        seen_in_mid == (mode == NestingMode::BottomUp)
            && top_seen_in_mid == (mode == NestingMode::TopDown),
        &format!(
            "{label}: effective members of the middle group follow the direction of the edges"
        ),
    );
}
