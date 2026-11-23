//! Rules: an asking AI and the objects that hang on a hidden group (memberships, invitations, requests, edges,
//! grants), the narrow view, the mirror and the change stream.
//!
//! Pattern: Template method (the second half of the hidden-group suite; `NotFound` of the right kind, never
//! `Forbidden`, which would tell that the object exists).

use super::cx::{Cx, err};
use super::rules_hidden::Hid;
use crate::groups_v1::shared::{Place, Revision};
use crate::groups_v1::{ChangeKind, Error, InvitationAnswer, Missing, NestingMode, RequestAnswer};
use crate::ids::group_ref;

fn missing<T>(r: &crate::groups_v1::Result<T>, want: fn(&Missing) -> bool) -> bool {
    matches!(err(r), Some(Error::NotFound(m)) if want(m))
}

pub(crate) async fn rules(cx: &mut Cx<'_>, h: &Hid, name: &str, before: Option<Place>) {
    let stale = Revision(999);
    let (m, i, g, e, r) = (h.member.0, h.invitation.0, h.grant.0, h.edge.0, h.request.0);
    let checks: [(&str, bool); 9] = [
        (
            "set_role",
            missing(&cx.g.set_role(m, stale, None).await, |x| {
                matches!(x, Missing::Membership(_))
            }),
        ),
        (
            "remove_member",
            missing(&cx.g.remove_member(m, stale).await, |x| {
                matches!(x, Missing::Membership(_))
            }),
        ),
        (
            "answer_nesting_request",
            missing(
                &cx.g
                    .answer_nesting_request(r, stale, RequestAnswer::Approve)
                    .await,
                |x| matches!(x, Missing::NestingRequest(_)),
            ),
        ),
        (
            "withdraw_nesting_request",
            missing(&cx.g.withdraw_nesting_request(r, stale).await, |x| {
                matches!(x, Missing::NestingRequest(_))
            }),
        ),
        (
            "change_nesting_mode",
            missing(
                &cx.g
                    .change_nesting_mode(e, stale, NestingMode::Single)
                    .await,
                |x| matches!(x, Missing::Nesting(_)),
            ),
        ),
        (
            "remove_nesting",
            missing(&cx.g.remove_nesting(e, stale).await, |x| {
                matches!(x, Missing::Nesting(_))
            }),
        ),
        (
            "answer_invitation",
            missing(
                &cx.g
                    .answer_invitation(i, stale, InvitationAnswer::Accept)
                    .await,
                |x| matches!(x, Missing::Invitation(_)),
            ),
        ),
        (
            "revoke_invitation",
            missing(&cx.g.revoke_invitation(i, stale).await, |x| {
                matches!(x, Missing::Invitation(_))
            }),
        ),
        (
            "revoke_grant",
            missing(&cx.g.revoke_grant(g, stale).await, |x| {
                matches!(x, Missing::Grant(_))
            }),
        ),
    ];
    for (method, ok) in checks {
        cx.ensure(
            ok,
            &format!(
                "hidden {name}: {method} of an object of a hidden group is NotFound of its own kind"
            ),
        );
    }
    narrow(cx, h, name).await;
    mirror(cx, h, name).await;
    // The change stream: what names the hidden group is left out, the rest stays.
    let Some(from) = before else { return };
    let page =
        cx.g.changes(Some(&from), 500)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    let names_hidden = |k: &ChangeKind| match k {
        ChangeKind::Group(x) | ChangeKind::Deleted(x) | ChangeKind::Restored(x) => *x == h.group,
        ChangeKind::Membership { group }
        | ChangeKind::Invitation { group }
        | ChangeKind::Grant { group }
        | ChangeKind::Mirror { group } => *group == h.group,
        ChangeKind::Nesting { child, parent } | ChangeKind::NestingRequest { child, parent } => {
            *child == h.group || *parent == h.group
        }
    };
    cx.ensure(
        page.iter().all(|c| !names_hidden(&c.kind)),
        &format!("hidden {name}: changes leaves out what names it"),
    );
    cx.ensure(
        page.iter()
            .any(|c| c.kind == ChangeKind::Membership { group: h.neighbour }),
        &format!("hidden {name}: changes keeps the rest"),
    );
}

/// The narrow view for other programs.
async fn narrow(cx: &mut Cx<'_>, h: &Hid, name: &str) {
    let Some(f) = cx.fx.facade() else {
        cx.skip("hidden: the narrow view (the fixture has none)");
        return;
    };
    let gref = group_ref(h.group);
    cx.ensure(
        matches!(f.describe(&gref).await, Ok(None)),
        &format!("hidden {name}: describe is None"),
    );
    cx.ensure(
        matches!(f.members_of(&gref).await, Err(Error::NotFound(_))),
        &format!("hidden {name}: members_of is NotFound"),
    );
    cx.ensure(
        f.memberships_of(&h.human)
            .await
            .is_ok_and(|v| !v.contains(&gref)),
        &format!("hidden {name}: memberships_of leaves it out"),
    );
}

/// An asking AI may not read the mirror at all.
async fn mirror(cx: &mut Cx<'_>, h: &Hid, name: &str) {
    let Some(mir) = cx.fx.mirroring() else {
        cx.skip("hidden: the mirror (the fixture has none)");
        return;
    };
    let forbidden = |r: &crate::groups_v1::Result<_>| matches!(err(r), Some(Error::Forbidden { group, .. }) if *group != h.group);
    cx.ensure(
        mir.mirror_status()
            .await
            .as_ref()
            .is_err_and(|e| matches!(e, Error::Forbidden { group, .. } if *group != h.group)),
        &format!("hidden {name}: mirror_status is Forbidden, without the id of a hidden group"),
    );
    cx.ensure(
        forbidden(&mir.group_mirror_state(h.group).await),
        &format!("hidden {name}: group_mirror_state is Forbidden, never with the hidden id"),
    );
    if let Some(dir) = cx.fx.directory() {
        let plan = mir.mirror_plan(dir).await;
        cx.ensure(matches!(plan, Err(crate::groups_v1::MirrorError::Groups(Error::Forbidden { group, .. })) if group != h.group), &format!("hidden {name}: mirror_plan is Forbidden"));
    }
}
