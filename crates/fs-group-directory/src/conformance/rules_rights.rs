//! Rules: grants, `may_do` with its reasons, `my_rights`, the change stream and the history.
//!
//! Pattern: Template method (a fixed list of rules; the people are made through the fixture).

use super::cx::{Cx, err};
use super::fixture::Kind;
use crate::groups_v1::shared::{PersonArt, Revision};
use crate::groups_v1::{ChangeKind, Error, Field, HistoryKind, Reason, Right};
use crate::ids::group_ref;

const ALL: [Right; 4] = [
    Right::Read,
    Right::AddMembers,
    Right::RemoveMembers,
    Right::Manage,
];

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    grants(cx).await;
    reasons(cx).await;
    feeds(cx).await;
}

async fn grants(cx: &mut Cx<'_>) {
    let (Some(g), Some(p), Some(q)) = (
        cx.group("grant").await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
    ) else {
        return;
    };
    let mine = cx.g.grants(g.id).await.unwrap_or_default();
    cx.ensure(
        mine.len() == 1 && mine[0].right == Right::Manage && mine[0].holder == cx.me,
        "grants: the creator holds Manage",
    );
    let Ok(grant) = cx.g.grant(g.id, &p, Right::AddMembers).await else {
        cx.ensure(false, "grant: the manager grants");
        return;
    };
    cx.ensure(
        grant.holder == p && grant.right == Right::AddMembers && grant.granted_by == cx.me,
        "grant: holder, right and who granted",
    );
    cx.ensure(matches!(err(&cx.g.grant(g.id, &p, Right::AddMembers).await), Some(Error::AlreadyThere(id)) if *id == grant.id), "grant: the same right to the same holder is AlreadyThere");
    let mut org = group_ref(g.id);
    org.art = PersonArt::Organization;
    cx.ensure(
        matches!(
            err(&cx.g.grant(g.id, &org, Right::Read).await),
            Some(Error::WrongArt {
                field: Field::Holder,
                ..
            })
        ),
        "grant: an organisation is not a holder (WrongArt)",
    );
    cx.ensure(
        cx.g.grant(g.id, &group_ref(g.id), Right::Read)
            .await
            .is_ok(),
        "grant: a group may be the holder",
    );
    if cx.act(&p).await {
        cx.ensure(
            cx.g.grants(g.id).await.is_ok(),
            "grants: a holder of any right reads them (Read)",
        );
        cx.ensure(
            matches!(
                err(&cx.g.grant(g.id, &q, Right::Read).await),
                Some(Error::Forbidden {
                    right: Right::Manage,
                    ..
                })
            ),
            "grant: without Manage it is Forbidden(Manage)",
        );
        cx.act_me().await;
    }
    cx.ensure(
        matches!(
            err(&cx
                .g
                .revoke_grant(grant.id, Revision(grant.revision.0 + 9))
                .await),
            Some(Error::Changed { .. })
        ),
        "revoke_grant: an old revision is Changed",
    );
    cx.ensure(
        cx.g.revoke_grant(grant.id, grant.revision).await.is_ok(),
        "revoke_grant: the manager takes it away",
    );
    cx.ensure(
        cx.g.grants(g.id)
            .await
            .is_ok_and(|v| v.iter().all(|x| x.id != grant.id)),
        "revoke_grant: it is gone",
    );
    cx.ensure(
        matches!(
            err(&cx.g.revoke_grant(grant.id, grant.revision).await),
            Some(Error::NotFound(_))
        ),
        "revoke_grant: twice is NotFound",
    );
    let h =
        cx.g.history(g.id, None, 50)
            .await
            .map_or_default(|p| p.items);
    cx.ensure(h.iter().any(|e| matches!(&e.kind, HistoryKind::Granted { holder, right: Right::AddMembers } if *holder == p)) && h.iter().any(|e| matches!(e.kind, HistoryKind::GrantRevoked { .. })), "history: Granted and GrantRevoked");
    cx.ensure(
        h.iter().all(|e| e.actor == cx.me),
        "history: the actor is who asked",
    );
}

async fn reasons(cx: &mut Cx<'_>) {
    let (Some(g), Some(p), Some(q), Some(r)) = (
        cx.group("may").await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
    ) else {
        return;
    };
    cx.g.grant(g.id, &p, Right::AddMembers).await.ok();
    cx.g.grant(g.id, &q, Right::Manage).await.ok();
    let d = cx.g.may_do(g.id, &p, Right::AddMembers).await;
    cx.ensure(
        d.is_ok_and(|d| d.allowed && matches!(d.reason, Reason::Holder { .. })),
        "may_do: a direct grant is Holder",
    );
    cx.ensure(
        cx.g.may_do(g.id, &p, Right::Read)
            .await
            .is_ok_and(|d| d.allowed),
        "may_do: every right covers Read",
    );
    cx.ensure(
        cx.g.may_do(g.id, &p, Right::RemoveMembers)
            .await
            .is_ok_and(|d| !d.allowed),
        "may_do: AddMembers is not RemoveMembers",
    );
    cx.ensure(
        cx.g.may_do(g.id, &p, Right::Manage)
            .await
            .is_ok_and(|d| !d.allowed),
        "may_do: AddMembers is not Manage",
    );
    for right in ALL {
        cx.ensure(
            cx.g.may_do(g.id, &q, right).await.is_ok_and(|d| d.allowed),
            "may_do: Manage covers every right",
        );
    }
    cx.ensure(
        cx.g.may_do(g.id, &r, Right::Read)
            .await
            .is_ok_and(|d| !d.allowed && d.reason == Reason::NoGrant),
        "may_do: no grant is NoGrant",
    );
    cx.ensure(
        matches!(
            err(&cx.g.may_do(g.id, &group_ref(g.id), Right::Read).await),
            Some(Error::WrongArt { .. })
        ),
        "may_do: a group is not asked about as a person",
    );
    let before =
        cx.g.history(g.id, None, 100)
            .await
            .map_or(0, |p| p.items.len());
    cx.g.may_do(g.id, &r, Right::Manage).await.ok();
    cx.ensure(
        cx.g.history(g.id, None, 100)
            .await
            .is_ok_and(|p| p.items.len() == before),
        "may_do: writes nothing",
    );
    if cx.act(&p).await {
        cx.ensure(
            cx.g.my_rights(g.id).await.is_ok_and(|v| {
                v.contains(&Right::AddMembers)
                    && v.contains(&Right::Read)
                    && !v.contains(&Right::Manage)
            }),
            "my_rights: what the asker may",
        );
        cx.act_me().await;
    }
}

async fn feeds(cx: &mut Cx<'_>) {
    let Ok(start) = cx.g.changes(None, 10).await else {
        return;
    };
    cx.ensure(
        start.items.is_empty(),
        "changes: None means from now, nothing yet",
    );
    let Some(g) = cx.group("feed").await else {
        return;
    };
    let Some(p) = cx.person(Kind::Human).await else {
        return;
    };
    let m = cx.member(g.id, &p).await;
    let Some(m) = m else { return };
    let Ok(page) = cx.g.changes(start.next.as_ref(), 100).await else {
        return;
    };
    let kinds: Vec<&ChangeKind> = page.items.iter().map(|c| &c.kind).collect();
    cx.ensure(
        kinds.iter().any(|k| **k == ChangeKind::Group(g.id))
            && kinds
                .iter()
                .any(|k| **k == ChangeKind::Membership { group: g.id }),
        "changes: the new group and its member are in the stream",
    );
    let first = page
        .items
        .iter()
        .position(|c| c.kind == ChangeKind::Group(g.id));
    let second = page
        .items
        .iter()
        .position(|c| c.kind == ChangeKind::Membership { group: g.id });
    cx.ensure(first < second, "changes: oldest first");
    let again = cx.g.changes(page.next.as_ref(), 100).await;
    cx.ensure(
        again.is_ok_and(|a| a.items.is_empty()),
        "changes: from the cursor on, nothing is repeated",
    );
    let small = cx.g.changes(start.next.as_ref(), 1).await;
    cx.ensure(
        small.as_ref().is_ok_and(|s| s.items.len() == 1),
        "changes: size limits the page",
    );
    cx.g.remove_member(m.id, m.revision).await.ok();
    let more =
        cx.g.changes(small.ok().and_then(|s| s.next).as_ref(), 100)
            .await
            .map_or_default(|p| p.items);
    cx.ensure(
        more.iter()
            .any(|c| c.kind == ChangeKind::Membership { group: g.id }),
        "changes: a later page goes on from the cursor",
    );
    let h =
        cx.g.history(g.id, None, 100)
            .await
            .map_or_default(|p| p.items);
    cx.ensure(
        h.windows(2).all(|w| w[0].at <= w[1].at),
        "history: oldest first",
    );
    let page = cx.g.history(g.id, None, 1).await;
    let Ok(page) = page else { return };
    let rest =
        cx.g.history(g.id, page.next.as_ref(), 100)
            .await
            .map_or_default(|p| p.items);
    cx.ensure(
        page.items.len() == 1 && page.next.is_some() && rest.len() + 1 == h.len(),
        "history: paging with a cursor",
    );
}
