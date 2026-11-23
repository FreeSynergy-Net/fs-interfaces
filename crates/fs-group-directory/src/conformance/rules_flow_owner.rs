//! Rules of `groups`: ownership flows over NO edge, in no direction and in no mode (ADR 0095: ownership is the
//! group's own; only members and rights flow, see `rules_directions.rs`).
//!
//! Pattern: Template method (the same check for each of the three modes; a mode that leaks owners fails its row).

use super::cx::Cx;
use super::fixture::Kind;
use super::rules_owner::owned;
use crate::groups::{GroupGovernance, NestingMode};

pub(crate) async fn rules(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    for (label, mode) in [
        ("bottom-up", NestingMode::BottomUp),
        ("top-down", NestingMode::TopDown),
        ("single", NestingMode::Single),
    ] {
        flows_no_owner(cx, v, label, mode).await;
    }
}

async fn flows_no_owner(cx: &mut Cx<'_>, v: &dyn GroupGovernance, label: &str, mode: NestingMode) {
    let (Some(top), Some(low)) = (
        owned(cx, v, &format!("{label} owner top")).await,
        owned(cx, v, &format!("{label} owner low")).await,
    ) else {
        return;
    };
    let Some(boss) = cx.person(Kind::Human).await else {
        return;
    };
    cx.ensure(
        v.appoint_owner(top.id, &boss).await.is_ok(),
        "setup: a second owner of the top group",
    );
    cx.link(low.id, top.id, mode).await;
    let on_low = v.owners(low.id).await.unwrap_or_default();
    cx.ensure(
        on_low.iter().all(|m| m.person != boss),
        &format!("{label}: an owner of the top group is not an owner of the low group"),
    );
    let on_top = v.owners(top.id).await.unwrap_or_default();
    cx.ensure(
        on_top.iter().any(|m| m.person == boss) && on_top.len() == 2,
        &format!("{label}: the owners of the top group stay its own"),
    );
}
