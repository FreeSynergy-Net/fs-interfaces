//! Rules of `groups`, roles over the member paths: the owner role only through the governance operations, the manager role only with `Manage`
//! (ADR 0095 Pkt. 3).
//!
//! Pattern: Template method (a fixed list of rules; each one is a mutant killer: drop the guard and a check fails).

use super::cx::{Cx, err};
use crate::groups::shared::{ObjectId, PersonRef};
use crate::groups::{Error, Membership, Right};
use crate::groups::{role_manager, role_owner};

/// The owner and manager roles are entries of the role book (ADR 0095 Pkt. 3), so the provider book knows both IRIs
/// (the fixture puts them in); the refusal must come from the rule, not from a role the book does not know.
/// The asker holds `Manage`. The owner role is made by the governance operations only, over no member path.
pub(super) async fn v1_role_paths(
    cx: &mut Cx<'_>,
    g: ObjectId,
    other: &PersonRef,
    cand: Option<&Membership>,
) {
    let owner = role_owner();
    cx.ensure(
        matches!(
            err(&cx.g.add_member(g, other, Some(&owner)).await),
            Some(Error::Forbidden { .. })
        ),
        "add_member: the owner role cannot be given over a member path",
    );
    cx.ensure(
        matches!(
            err(&cx.g.invite(g, other, Some(&owner), None).await),
            Some(Error::Forbidden { .. })
        ),
        "invite: the owner role cannot be offered over a member path",
    );
    if let Some(m) = cand {
        cx.ensure(
            matches!(
                err(&cx.g.set_role(m.id, m.revision, Some(&owner)).await),
                Some(Error::Forbidden { .. })
            ),
            "set_role: a manager cannot make a member an owner (member path)",
        );
    }
}

/// Whoever only holds `AddMembers` cannot hand out the manager role (it gives `Manage`).
pub(super) async fn adder_cannot_make_managers(
    cx: &mut Cx<'_>,
    g: ObjectId,
    adder: &PersonRef,
    other: &PersonRef,
) {
    cx.ensure(
        cx.g.grant(g, adder, Right::AddMembers).await.is_ok(),
        "setup: a holder of AddMembers",
    );
    if cx.act(adder).await {
        let manager = role_manager();
        cx.ensure(
            matches!(
                err(&cx.g.add_member(g, other, Some(&manager)).await),
                Some(Error::Forbidden { .. })
            ),
            "add_member: AddMembers does not give the manager role",
        );
        cx.ensure(
            matches!(
                err(&cx.g.invite(g, other, Some(&manager), None).await),
                Some(Error::Forbidden { .. })
            ),
            "invite: AddMembers does not offer the manager role",
        );
    }
}
