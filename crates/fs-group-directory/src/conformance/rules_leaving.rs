//! Rules: leaving, `LastManager` and paging of the member lists.
//!
//! Pattern: Template method (a fixed list of rules; each rule seeds its own data through the trait).

use super::cx::{Cx, err};
use super::fixture::Kind;
use crate::groups_v1::{Error, NestingMode, Right};
use crate::ids::group_ref;

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    leaving(cx).await;
    paging(cx).await;
}

async fn leaving(cx: &mut Cx<'_>) {
    let (Some(g), Some(p), Some(q)) = (
        cx.group("leave").await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
    ) else {
        return;
    };
    let (Some(mp), Some(mq)) = (cx.member(g.id, &p).await, cx.member(g.id, &q).await) else {
        return;
    };
    if cx.act(&p).await {
        cx.ensure(
            matches!(
                err(&cx.g.remove_member(mq.id, mq.revision).await),
                Some(Error::Forbidden {
                    right: Right::RemoveMembers,
                    ..
                })
            ),
            "remove_member: another member without the right is Forbidden(RemoveMembers)",
        );
        cx.ensure(
            cx.g.remove_member(mp.id, mp.revision).await.is_ok(),
            "remove_member: a person leaves on her own",
        );
        cx.act_me().await;
    }
    // LastManager: the only holder of Manage cannot be revoked or removed.
    let (Some(target), Some(holders), Some(boss)) = (
        cx.group("lm target").await,
        cx.group("lm holders").await,
        cx.person(Kind::Human).await,
    ) else {
        return;
    };
    let Some(boss_member) = cx.member(holders.id, &boss).await else {
        return;
    };
    let mine = cx.g.grants(target.id).await.unwrap_or_default();
    let Some(own) = mine.iter().find(|x| x.right == Right::Manage) else {
        return;
    };
    cx.ensure(
        matches!(
            err(&cx.g.revoke_grant(own.id, own.revision).await),
            Some(Error::LastManager)
        ),
        "revoke_grant: the last Manage holder is LastManager",
    );
    cx.ensure(
        cx.g.grant(target.id, &group_ref(holders.id), Right::Manage)
            .await
            .is_ok(),
        "grant: Manage to a group",
    );
    cx.ensure(
        cx.g.revoke_grant(own.id, own.revision).await.is_ok(),
        "revoke_grant: not the last any more",
    );
    cx.ensure(
        matches!(
            err(&cx
                .g
                .remove_member(boss_member.id, boss_member.revision)
                .await),
            Some(Error::LastManager)
        ),
        "remove_member: removing the last way to Manage is LastManager",
    );
}

async fn paging(cx: &mut Cx<'_>) {
    let (Some(a), Some(b)) = (cx.group("pm a").await, cx.group("pm b").await) else {
        return;
    };
    cx.link(a.id, b.id, NestingMode::BottomUp).await;
    for _ in 0..3 {
        let Some(p) = cx.person(Kind::Human).await else {
            return;
        };
        cx.member(a.id, &p).await;
    }
    let first = cx.g.members(a.id, None, 2).await;
    let Ok(first) = first else { return };
    cx.ensure(
        first.items.len() == 2 && first.next.is_some(),
        "members: a page of 2 has a next",
    );
    let rest =
        cx.g.members(a.id, first.next.as_ref(), 5)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    cx.ensure(
        rest.len() == 1 && first.items.iter().all(|m| m.id != rest[0].id),
        "members: the next page has the rest, none twice",
    );
    let eff = cx.g.effective_members(b.id, None, 2).await;
    let Ok(eff) = eff else { return };
    cx.ensure(
        eff.items.len() == 2 && eff.next.is_some(),
        "effective_members: a page of 2 has a next",
    );
    let rest =
        cx.g.effective_members(b.id, eff.next.as_ref(), 5)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    cx.ensure(
        rest.len() == 1,
        "effective_members: the stable order, the rest on the next page",
    );
}
