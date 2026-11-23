//! Rules: members, roles, visibility of the member list, paging, invitations.
//!
//! Pattern: Template method (a fixed list of rules; the people are made through the fixture).

use super::cx::{Cx, err};
use super::fixture::Kind;
use crate::groups::shared::Revision;
use crate::groups::{Error, Field, InForce, Right, Visibility};
use crate::ids::{group_ref, person};

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    members(cx).await;
    roles(cx).await;
    visibility(cx).await;
    super::rules_invites::rules(cx).await;
    super::rules_leaving::rules(cx).await;
}

async fn members(cx: &mut Cx<'_>) {
    let Some(g) = cx.group("mem").await else {
        return;
    };
    let Some(p) = cx.person(Kind::Human).await else {
        return;
    };
    let Some(m) = cx.member(g.id, &p).await else {
        return;
    };
    cx.ensure(
        m.group == g.id
            && m.person == p
            && m.role.is_none()
            && m.in_force == InForce::Yes
            && m.from_invitation.is_none(),
        "add_member: a plain member in force",
    );
    cx.ensure(matches!(err(&cx.g.add_member(g.id, &p, None).await), Some(Error::AlreadyThere(id)) if *id == m.id), "add_member: the same person with the same role is AlreadyThere with the id");
    cx.ensure(
        matches!(
            err(&cx.g.add_member(g.id, &group_ref(g.id), None).await),
            Some(Error::WrongArt {
                field: Field::Person,
                ..
            })
        ),
        "add_member: a group is never a member (WrongArt)",
    );
    cx.ensure(
        matches!(
            err(&cx.g.add_member(g.id, &person(8_000_001), None).await),
            Some(Error::NotFound(_))
        ),
        "add_member: a person people do not know is NotFound",
    );
    cx.ensure(
        cx.g.members(g.id, None, 10)
            .await
            .is_ok_and(|v| v.items.len() == 2),
        "members: the direct members (the owner and the member)",
    );
    cx.ensure(
        cx.g.groups_of(&p)
            .await
            .is_ok_and(|v| v.len() == 1 && v[0].group == g.id && v[0].direct == Some(m.id)),
        "groups_of: the group with the direct membership",
    );
    cx.ensure(
        matches!(
            err(&cx.g.groups_of(&group_ref(g.id)).await),
            Some(Error::WrongArt { .. })
        ),
        "groups_of: a group as person is WrongArt",
    );
    cx.ensure(
        cx.g.remove_member(m.id, Revision(m.revision.0 + 9))
            .await
            .is_err(),
        "remove_member: an old revision fails",
    );
    cx.ensure(
        cx.g.remove_member(m.id, m.revision).await.is_ok()
            && cx.g.groups_of(&p).await.is_ok_and(|v| v.is_empty()),
        "remove_member: the manager removes a member",
    );
}

async fn roles(cx: &mut Cx<'_>) {
    let Some((plain, _)) = cx.fx.roles().await else {
        cx.skip("roles: the fixture has no roles");
        return;
    };
    let (Some(g), Some(p)) = (cx.group("role").await, cx.person(Kind::Human).await) else {
        return;
    };
    let Some(m) = cx.member(g.id, &p).await else {
        return;
    };
    let Ok(with) = cx.g.set_role(m.id, m.revision, Some(&plain)).await else {
        cx.ensure(false, "set_role: the manager sets a role");
        return;
    };
    cx.ensure(
        with.role.as_ref() == Some(&plain) && with.revision > m.revision,
        "set_role: the role is set, the revision rises",
    );
    cx.ensure(
        matches!(
            err(&cx.g.set_role(m.id, m.revision, None).await),
            Some(Error::Changed { .. })
        ),
        "set_role: an old revision is Changed",
    );
    cx.ensure(
        cx.g.add_member(g.id, &p, None).await.is_ok(),
        "add_member: the same person with another role is a second membership",
    );
    cx.ensure(
        matches!(
            err(&cx.g.add_member(g.id, &p, Some(&plain)).await),
            Some(Error::AlreadyThere(_))
        ),
        "add_member: the same role again is AlreadyThere",
    );
    cx.ensure(
        cx.g.set_role(m.id, with.revision, None)
            .await
            .is_ok_and(|x| x.role.is_none()),
        "set_role: None takes the role away",
    );
}

async fn visibility(cx: &mut Cx<'_>) {
    let (Some(m), Some(other)) = (cx.person(Kind::Human).await, cx.person(Kind::Human).await)
    else {
        return;
    };
    for (vis, label) in [
        (Visibility::Managers, "managers"),
        (Visibility::Members, "members"),
    ] {
        let mut d = cx.draft(label);
        d.visibility = vis;
        let Some(g) = cx.group_from(d).await else {
            return;
        };
        cx.member(g.id, &m).await;
        cx.ensure(
            cx.g.members(g.id, None, 5).await.is_ok(),
            "visibility: the manager reads the list",
        );
        if cx.act(&other).await {
            let r = cx.g.members(g.id, None, 5).await;
            cx.ensure(
                matches!(
                    err(&r),
                    Some(Error::Forbidden {
                        right: Right::Read,
                        ..
                    })
                ),
                "visibility: a stranger is Forbidden(Read)",
            );
            cx.ensure(
                matches!(
                    err(&cx.g.effective_members(g.id, None, 5).await),
                    Some(Error::Forbidden {
                        right: Right::Read,
                        ..
                    })
                ),
                "visibility: the effective list follows the same rule",
            );
            cx.ensure(
                cx.g.list(
                    &crate::groups::GroupQuery {
                        text: g.name.clone(),
                        parent: crate::groups::ParentFilter::Any,
                        member: None,
                        include_deleted: false,
                    },
                    None,
                    5,
                )
                .await
                .is_ok_and(|p| p.items.iter().all(|s| s.direct_members.is_none())),
                "visibility: list shows no member number where the list is hidden",
            );
        }
        if vis == Visibility::Members && cx.act(&m).await {
            cx.ensure(
                cx.g.members(g.id, None, 5).await.is_ok(),
                "visibility: a member reads the list of Members",
            );
        }
        cx.act_me().await;
    }
}
