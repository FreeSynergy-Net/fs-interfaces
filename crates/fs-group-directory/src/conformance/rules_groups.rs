//! Rules: creating, reading, updating, listing and the trash.
//!
//! Pattern: Template method (a fixed list of rules; each rule seeds its own groups through the trait).

use super::cx::{Cx, err};
use super::fixture::Kind;
use crate::groups_v1::shared::Revision;
use crate::groups_v1::{Error, Field, GroupPatch, HistoryKind, NestingSide, ProblemKind, Right};
use crate::ids::{object_id, reference_of};

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    create(cx).await;
    read(cx).await;
    update(cx).await;
    super::rules_trash::rules(cx).await;
}

fn invalid(e: Option<&Error>, field: Field, problem: &ProblemKind) -> bool {
    matches!(e, Some(Error::Invalid(p)) if p.iter().any(|x| x.field == field && &x.problem == problem))
}

async fn create(cx: &mut Cx<'_>) {
    let Some(g) = cx.group("create").await else {
        return;
    };
    cx.ensure(
        g.reference == reference_of(g.id),
        "create: the reference is urn:uuid of the id",
    );
    cx.ensure(
        !g.name.is_empty() && g.revision.0 > 0,
        "create: name and revision are set",
    );
    cx.ensure(
        cx.g.get(g.id).await.ok().flatten().as_ref() == Some(&g),
        "create: get returns what create returned",
    );
    cx.ensure(
        cx.g.get_by_reference(&g.reference)
            .await
            .ok()
            .flatten()
            .as_ref()
            == Some(&g),
        "create: get_by_reference finds it",
    );
    cx.ensure(
        cx.g.my_rights(g.id)
            .await
            .is_ok_and(|r| r.contains(&Right::Manage)),
        "create: the asker gets Manage",
    );
    let h =
        cx.g.history(g.id, None, 10)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    cx.ensure(
        h.first().is_some_and(|e| e.kind == HistoryKind::Created),
        "create: the history starts with Created",
    );
    let mut d = cx.draft("bad");
    d.name = String::from("  ");
    cx.ensure(
        invalid(err(&cx.g.create(d).await), Field::Name, &ProblemKind::Empty),
        "create: an empty name is Invalid(Name, Empty)",
    );
    let mut d = cx.draft("bad");
    d.name = "x".repeat(2000);
    cx.ensure(matches!(err(&cx.g.create(d).await), Some(Error::Invalid(p)) if p.iter().any(|x| matches!(x.problem, ProblemKind::TooLong { .. }))), "create: a too long name is Invalid(TooLong)");
    let mut d = cx.draft("bad");
    (d.sensitive, d.mirror) = (true, true);
    cx.ensure(
        invalid(
            err(&cx.g.create(d).await),
            Field::Mirror,
            &ProblemKind::SensitiveNotMirrored,
        ),
        "create: mirror with sensitive is Invalid",
    );
    let mut d = cx.draft("bad");
    (d.ai, d.mirror) = (true, true);
    cx.ensure(
        invalid(
            err(&cx.g.create(d).await),
            Field::Mirror,
            &ProblemKind::AiNotMirrored,
        ),
        "create: mirror with ai is Invalid",
    );
}

async fn read(cx: &mut Cx<'_>) {
    let none = object_id(8_888_888);
    cx.ensure(
        matches!(cx.g.get(none).await, Ok(None)),
        "get: an unknown id is None",
    );
    cx.ensure(
        matches!(cx.g.get_by_reference(&reference_of(none)).await, Ok(None)),
        "get_by_reference: an unknown reference is None",
    );
    cx.ensure(
        matches!(
            err(&cx.g.members(none, None, 5).await),
            Some(Error::NotFound(_))
        ),
        "members: an unknown group is NotFound",
    );
    cx.ensure(
        matches!(
            err(&cx.g.update(none, Revision(1), GroupPatch::default()).await),
            Some(Error::NotFound(_))
        ),
        "update: an unknown group is NotFound",
    );
    cx.ensure(
        matches!(
            err(&cx.g.delete(none, Revision(1)).await),
            Some(Error::NotFound(_))
        ),
        "delete: an unknown group is NotFound",
    );
    cx.ensure(
        matches!(
            err(&cx.g.nestings(none, NestingSide::AsChild).await),
            Some(Error::NotFound(_))
        ),
        "nestings: an unknown group is NotFound",
    );
}

async fn update(cx: &mut Cx<'_>) {
    let Some(g) = cx.group("update").await else {
        return;
    };
    let patch = GroupPatch {
        name: Some("Zzconf renamed".to_owned()),
        kind: Some(Some("Team".to_owned())),
        ..GroupPatch::default()
    };
    let Ok(new) = cx.g.update(g.id, g.revision, patch).await else {
        cx.ensure(false, "update: the owner may update");
        return;
    };
    cx.ensure(
        new.name == "Zzconf renamed"
            && new.kind.as_deref() == Some("Team")
            && new.revision > g.revision,
        "update: fields change, the revision rises",
    );
    let stale =
        cx.g.update(
            g.id,
            g.revision,
            GroupPatch {
                name: Some("Zzconf stale".to_owned()),
                ..GroupPatch::default()
            },
        )
        .await;
    cx.ensure(
        matches!(err(&stale), Some(Error::Changed { current }) if *current == new.revision),
        "update: an old revision is Changed with the current one",
    );
    let clear =
        cx.g.update(
            g.id,
            new.revision,
            GroupPatch {
                kind: Some(None),
                ..GroupPatch::default()
            },
        )
        .await;
    cx.ensure(
        clear.is_ok_and(|c| c.kind.is_none()),
        "update: Some(None) clears an optional field",
    );
    let rev = cx.rev(g.id).await;
    let bad =
        cx.g.update(
            g.id,
            rev,
            GroupPatch {
                name: Some(String::new()),
                ..GroupPatch::default()
            },
        )
        .await;
    cx.ensure(
        invalid(err(&bad), Field::Name, &ProblemKind::Empty),
        "update: an empty name is Invalid",
    );
    let bad =
        cx.g.update(
            g.id,
            rev,
            GroupPatch {
                sensitive: Some(true),
                mirror: Some(true),
                ..GroupPatch::default()
            },
        )
        .await;
    cx.ensure(
        matches!(err(&bad), Some(Error::Invalid(_))),
        "update: sensitive with mirror is Invalid",
    );
    let h =
        cx.g.history(g.id, None, 50)
            .await
            .map(|p| p.items)
            .unwrap_or_default();
    cx.ensure(
        h.iter()
            .any(|e| matches!(e.kind, HistoryKind::Updated { .. })),
        "update: the history has Updated",
    );
    if let Some(other) = cx.person(Kind::Human).await
        && cx.act(&other).await
    {
        let rev = cx.rev(g.id).await;
        let r =
            cx.g.update(
                g.id,
                rev,
                GroupPatch {
                    name: Some("Zzconf nope".to_owned()),
                    ..GroupPatch::default()
                },
            )
            .await;
        cx.ensure(
            matches!(
                err(&r),
                Some(Error::Forbidden {
                    right: Right::Manage,
                    ..
                })
            ),
            "update: without Manage it is Forbidden(Manage)",
        );
        cx.ensure(
            matches!(
                err(&cx.g.delete(g.id, rev).await),
                Some(Error::Forbidden {
                    right: Right::Manage,
                    ..
                })
            ),
            "delete: without Manage it is Forbidden(Manage)",
        );
        cx.ensure(
            cx.g.get(g.id).await.is_ok_and(|x| x.is_some()),
            "get: everyone may read name and kind",
        );
        cx.act_me().await;
    }
}
