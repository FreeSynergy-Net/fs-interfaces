//! Rules of `groups`, addons: attach, several per group, open IRI, who may, independence, self-management.
//!
//! Pattern: Template method (a fixed list of rules; each one is a mutant killer).

use super::cx::{Cx, err};
use super::fixture::Kind;
use super::rules_owner::owned;
use crate::groups::shared::{BasicValue as _, Reference};
use crate::groups::{AddonKind, FormalOrganization, GroupGovernance, ORG_FORMAL_ORGANIZATION};
use crate::groups::{Error, Right};

pub(super) fn formal(name: &str) -> AddonKind {
    AddonKind::FormalOrganization(FormalOrganization {
        legal_name: name.to_owned(),
        legal_form: Some("Verein".to_owned()),
        registration: None,
    })
}

pub(super) fn other() -> AddonKind {
    AddonKind::Other(Reference::parse("https://example.org/ns/addons/tax-exempt").expect("an IRI"))
}

fn denied<T>(r: &Result<T, Error>) -> bool {
    matches!(r, Err(Error::NeedsLevel { .. }))
}

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    let Some(v) = cx.gov else { return };
    attach(cx, v).await;
    rights(cx, v).await;
    super::rules_addon_more::rules(cx, v).await;
    cx.act_me().await;
}

async fn attach(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    let Some(g) = owned(cx, v, "add base").await else {
        return;
    };
    cx.ensure(
        v.addons(g.id).await.is_ok_and(|a| a.is_empty()),
        "addons: a new group has none (the core is the group)",
    );
    let a = v.attach_addon(g.id, formal("Zzconf e.V.")).await;
    cx.ensure(
        a.as_ref()
            .is_ok_and(|a| a.group == g.id && a.attached_by == cx.me),
        "attach_addon: attached, by the connection",
    );
    cx.ensure(
        v.attach_addon(g.id, other()).await.is_ok(),
        "attach_addon: several addons per group, open by IRI",
    );
    cx.ensure(
        v.addons(g.id).await.is_ok_and(|l| l.len() == 2),
        "addons: both are listed",
    );
    cx.ensure(
        matches!(
            err(&v.attach_addon(g.id, formal("Zzconf zwei")).await),
            Some(Error::AddonAlreadyThere(_))
        ),
        "attach_addon: the same IRI twice is AddonAlreadyThere",
    );
    cx.ensure(
        AddonKind::FormalOrganization(FormalOrganization {
            legal_name: String::new(),
            legal_form: None,
            registration: None,
        })
        .iri()
        .canonical()
            == ORG_FORMAL_ORGANIZATION,
        "the formal organisation is org:FormalOrganization (W3C ORG)",
    );
    let bad = v
        .attach_addon(
            g.id,
            AddonKind::Other(Reference::parse("https://example.org/ns/addons/x").expect("an IRI")),
        )
        .await;
    cx.ensure(bad.is_ok(), "attach_addon: any other IRI is a valid addon");
    let mut empty = v.addons(g.id).await.unwrap_or_default();
    empty.retain(|a| matches!(a.kind, AddonKind::FormalOrganization(_)));
    let Some(f) = empty.pop() else { return };
    cx.ensure(
        matches!(
            err(&v
                .update_addon(
                    f.id,
                    crate::groups::shared::Revision(f.revision.0 + 5),
                    formal("Zzconf neu")
                )
                .await),
            Some(Error::Changed { .. })
        ),
        "update_addon: a stale revision is Changed",
    );
    cx.ensure(
        matches!(
            err(&v.update_addon(f.id, f.revision, formal("")).await),
            Some(Error::Invalid(_))
        ),
        "update_addon: an empty legal name is Invalid",
    );
    let upd = v.update_addon(f.id, f.revision, formal("Zzconf neu")).await;
    cx.ensure(
        upd.as_ref().is_ok_and(|a| a.revision.0 == f.revision.0 + 1),
        "update_addon: revision moves",
    );
    let Ok(upd) = upd else { return };
    cx.ensure(
        v.detach_addon(upd.id, upd.revision).await.is_ok(),
        "detach_addon: detached",
    );
    cx.ensure(
        v.addons(g.id)
            .await
            .is_ok_and(|l| l.len() == 2 && l.iter().all(|a| a.id != upd.id)),
        "addons: the detached one is gone",
    );
    let rev = cx.rev(g.id).await;
    cx.ensure(cx.g.delete(g.id, rev).await.is_ok(), "setup: delete");
    cx.ensure(cx.g.restore(g.id).await.is_ok(), "setup: restore");
    cx.ensure(
        v.addons(g.id).await.is_ok_and(|l| l.len() == 2),
        "addons: they come back with the restored group",
    );
}

async fn rights(cx: &mut Cx<'_>, v: &dyn GroupGovernance) {
    let Some(g) = owned(cx, v, "add rights").await else {
        return;
    };
    let (Some(stranger), Some(plain), Some(mgr)) = (
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
    ) else {
        return;
    };
    cx.member(g.id, &plain).await;
    cx.ensure(
        cx.g.grant(g.id, &mgr, Right::Manage).await.is_ok(),
        "setup: manager",
    );
    cx.ensure(
        v.attach_addon(g.id, formal("Zzconf rights")).await.is_ok(),
        "setup: attach",
    );
    let Some(a) = v.addons(g.id).await.ok().and_then(|mut l| l.pop()) else {
        return;
    };
    for (who, label) in [(&stranger, "a stranger"), (&plain, "a plain member")] {
        if cx.act(who).await {
            cx.ensure(
                denied(&v.attach_addon(g.id, other()).await),
                &format!("attach_addon: {label} is refused (NeedsLevel)"),
            );
            cx.ensure(
                denied(&v.update_addon(a.id, a.revision, formal("Zzconf x")).await),
                &format!("update_addon: {label} is refused"),
            );
            cx.ensure(
                denied(&v.detach_addon(a.id, a.revision).await),
                &format!("detach_addon: {label} is refused"),
            );
        }
    }
    cx.act_me().await;
    cx.ensure(
        v.addons(g.id).await.is_ok_and(|l| l.len() == 1),
        "addons: nothing changed by the refused calls",
    );
    if cx.act(&mgr).await {
        cx.ensure(
            v.attach_addon(g.id, other()).await.is_ok(),
            "attach_addon: a manager may",
        );
        cx.act_me().await;
    }
    // a foreign group of the stranger is not reachable through the addon of this one
    if cx.act(&stranger).await {
        let theirs = owned(cx, v, "add theirs").await;
        if let Some(t) = theirs {
            cx.ensure(
                v.update_addon(a.id, a.revision, formal("Zzconf y"))
                    .await
                    .is_err()
                    && v.attach_addon(t.id, other()).await.is_ok(),
                "attach_addon: she writes her own group only",
            );
        }
        cx.act_me().await;
    }
}
