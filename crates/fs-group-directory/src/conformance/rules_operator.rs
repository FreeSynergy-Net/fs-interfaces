//! Rules of the operators, part 2: NO way of the contract reaches the operator (user decision 16): not the member
//! paths, not the scope, not the rights, not an intervention, not the Project scope.
//!
//! Pattern: Template method (each rule is a mutant killer: open one contract path and a check fails).

use super::cx::{Cx, err};
use super::fixture::Kind;
use super::rules_operator_local::{listed, rules as local_rules};
use super::rules_owner::owned;
use crate::groups::shared::{ObjectId, PersonRef};
use crate::groups::{Error, GroupGovernance, Right, Scope, role_operator};

fn forbidden<T>(r: &Result<T, Error>) -> bool {
    matches!(r, Err(Error::Forbidden { .. }))
}

fn denied<T>(r: &Result<T, Error>) -> bool {
    matches!(r, Err(Error::ScopeDenied(_)))
}

/// The operator group of the provider (made by the first local command).
async fn operator_group(v: &dyn GroupGovernance) -> Option<ObjectId> {
    v.operators().await.ok().and_then(|l| l.group)
}

pub(crate) async fn rules(cx: &mut Cx<'_>) {
    let Some(v) = cx.gov else { return };
    let Some(local) = cx.fx.local_operator() else {
        cx.skip("operators: the fixture offers no LocalOperator");
        return;
    };
    cx.act_me().await;
    let Some(p) = local_rules(cx, v, local).await else {
        return;
    };
    let Some(og) = operator_group(v).await else {
        cx.ensure(false, "operators: the group Betreiber exists");
        return;
    };
    let (Some(q), Some(own)) = (cx.person(Kind::Human).await, owned(cx, "op own").await) else {
        return;
    };
    let role = role_operator();
    let Some(m) = cx.member(own.id, &q).await else {
        return;
    };
    // member paths in a group she owns
    cx.ensure(
        forbidden(&cx.g.add_member(own.id, &q, Some(&role)).await),
        "add_member: the role operator is never given over the contract",
    );
    cx.ensure(
        forbidden(&cx.g.set_role(m.id, m.revision, Some(&role)).await),
        "set_role: the role operator is never given over the contract",
    );
    cx.ensure(
        forbidden(&cx.g.invite(own.id, &q, Some(&role), None).await),
        "invite: the role operator is never offered over the contract",
    );
    // the group Betreiber itself
    cx.ensure(
        forbidden(&cx.g.add_member(og, &q, Some(&role)).await)
            && forbidden(&cx.g.add_member(og, &q, None).await),
        "add_member: nobody is added to the group Betreiber over the contract",
    );
    cx.ensure(
        forbidden(&cx.g.invite(og, &q, None, None).await),
        "invite: nobody is invited to the group Betreiber",
    );
    cx.ensure(
        forbidden(&cx.g.grant(og, &q, Right::Manage).await)
            && forbidden(&cx.g.grant(og, &q, Right::AddMembers).await),
        "grant: no right on the group Betreiber is handed out",
    );
    cx.ensure(
        forbidden(
            &cx.g
                .request_nesting(own.id, og, crate::groups::NestingMode::BottomUp)
                .await,
        ) && forbidden(
            &cx.g
                .request_nesting(og, own.id, crate::groups::NestingMode::TopDown)
                .await,
        ),
        "request_nesting: the group Betreiber takes no edge",
    );
    let rev = cx.rev(og).await;
    cx.ensure(
        err(&cx.g.delete(og, rev).await).is_some(),
        "delete: the group Betreiber is not deleted over the contract",
    );
    cx.ensure(
        matches!(
            cx.g.my_rights(og).await.map(|r| r.contains(&Right::Manage)),
            Ok(false)
        ),
        "my_rights: nobody holds Manage at the group Betreiber",
    );
    // the operator herself cannot leave or change her role over the contract
    let rows = cx.g.members(og, None, 100).await.map(|p| p.items);
    let hers = rows
        .as_ref()
        .ok()
        .and_then(|l| l.iter().find(|x| x.person == p).cloned());
    cx.ensure(
        hers.is_some(),
        "members: the operator is a member of Betreiber",
    );
    if let Some(h) = hers {
        cx.ensure(
            forbidden(&cx.g.set_role(h.id, h.revision, None).await),
            "set_role: the operator role is not taken away over the contract",
        );
        if cx.act(&p).await {
            cx.ensure(
                forbidden(&cx.g.remove_member(h.id, h.revision).await),
                "remove_member: an operator does not leave over the contract either (only the local command)",
            );
        }
        cx.act_me().await;
    }
    // the scope
    cx.ensure(
        denied(&v.grant_scope(own.id, Scope::Server).await),
        "grant_scope: Server is never handed out over the contract",
    );
    cx.ensure(
        !listed(v, &q).await && !listed(v, &cx.me.clone()).await,
        "operators: nobody got there over the contract",
    );
    cx.ensure(
        listed(v, &p).await,
        "operators: the operator is still the one the local command made",
    );
    cx.act_me().await;
}

/// `x` holds the `Project` scope and is the asker: no scope and no intervention makes her an operator (user
/// decision 16: operator and Project administration are two separate roles).
pub(crate) async fn by_project_holder(cx: &mut Cx<'_>, v: &dyn GroupGovernance, x: &PersonRef) {
    let Some(local) = cx.fx.local_operator() else {
        return;
    };
    cx.act_me().await;
    if operator_group(v).await.is_none() {
        let Some(first) = cx.person(Kind::Human).await else {
            return;
        };
        cx.ensure(
            local.operator_set_local(&first).await.is_ok(),
            "setup: the local command makes an operator",
        );
    }
    let Some(og) = operator_group(v).await else {
        return;
    };
    cx.act(x).await;
    cx.ensure(
        denied(
            &v.open_intervention(og, Scope::Project, "Zzconf: Betreiber werden")
                .await,
        ),
        "open_intervention: a Project holder does not intervene in the group Betreiber",
    );
    cx.ensure(
        forbidden(&cx.g.add_member(og, x, Some(&role_operator())).await),
        "add_member: a Project holder does not make herself an operator",
    );
    cx.ensure(
        denied(&v.open_intervention(og, Scope::Server, "Zzconf").await),
        "open_intervention: Server is no scope of an intervention",
    );
    let wanted_group = cx.group("op project holder").await;
    if let Some(g) = wanted_group {
        cx.ensure(
            denied(&v.grant_scope(g.id, Scope::Server).await),
            "grant_scope: a Project holder does not hand out Server",
        );
    }
    let server = v
        .scopes()
        .await
        .ok()
        .and_then(|s| s.into_iter().find(|g| g.scope == Scope::Server));
    if let Some(s) = server {
        cx.ensure(
            denied(&v.revoke_scope(s.id).await),
            "revoke_scope: the Server scope is not revoked over the contract",
        );
    }
    cx.ensure(
        !listed(v, x).await,
        "operators: a Project holder is not an operator",
    );
    cx.act_me().await;
}
