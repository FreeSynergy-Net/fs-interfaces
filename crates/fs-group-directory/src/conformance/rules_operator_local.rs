//! Rules of the operators, part 1: the LOCAL way in (`LocalOperator`) and the read side (`operators`): who may become
//! an operator (a known, unsupervised human, never an AI, an unknown or a supervised account, never a profile or a
//! group), the list is open to everybody, and a mark that changes later takes the role away at once.
//!
//! Pattern: Template method (each rule is a mutant killer: drop one check of `check_operator_candidate` and it fails).

use super::cx::{Cx, err};
use super::fixture::Kind;
use crate::groups::shared::{PersonArt, PersonRef};
use crate::groups::{Error, GroupGovernance, LocalOperator};

/// Whether `who` is on the list of operators as the asker reads it.
pub(super) async fn listed(v: &dyn GroupGovernance, who: &PersonRef) -> bool {
    v.operators()
        .await
        .is_ok_and(|l| l.persons.iter().any(|p| p == who))
}

/// Runs the rules; returns the operator it made (the contract rules need one), `None` when a setup failed.
pub(super) async fn rules(
    cx: &mut Cx<'_>,
    v: &dyn GroupGovernance,
    local: &dyn LocalOperator,
) -> Option<PersonRef> {
    let (Some(p), Some(agent), Some(unknown), Some(sup), Some(unsup), Some(q)) = (
        cx.person(Kind::Human).await,
        cx.person(Kind::Agent).await,
        cx.person(Kind::Unknown).await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
        cx.person(Kind::Human).await,
    ) else {
        return None;
    };
    cx.ensure(
        cx.fx.supervise(&sup, Some(true)).await && cx.fx.supervise(&unsup, None).await,
        "setup: a supervised account and one of unknown supervision",
    );
    let refused = [
        (&agent, "an AI account"),
        (&unknown, "an account of unknown kind"),
        (&sup, "a supervised account"),
        (&unsup, "an account whose supervision is not known"),
    ];
    for (who, what) in refused {
        let r = local.operator_set_local(who).await;
        cx.ensure(
            err(&r).is_some_and(|e| {
                matches!(
                    e,
                    Error::AgentNotAdmin | Error::AiStatusUnknown | Error::SupervisedCannotOperate
                )
            }),
            &format!("operator_set_local: {what} is never an operator"),
        );
        cx.ensure(
            !listed(v, who).await,
            &format!("operators: {what} is not on the list"),
        );
    }
    for art in [PersonArt::Group, PersonArt::Profile] {
        let not_account = PersonRef { art, ..p.clone() };
        cx.ensure(
            matches!(
                err(&local.operator_set_local(&not_account).await),
                Some(Error::WrongArt { .. })
            ),
            "operator_set_local: only an account (no profile, no group) can be an operator",
        );
    }
    cx.ensure(
        local.operator_set_local(&p).await.is_ok(),
        "operator_set_local: a known human becomes an operator",
    );
    cx.ensure(
        matches!(
            err(&local.operator_set_local(&p).await),
            Some(Error::AlreadyThere(_))
        ),
        "operator_set_local: twice is AlreadyThere",
    );
    cx.ensure(
        v.operators().await.is_ok_and(|l| l.group.is_some()) && listed(v, &p).await,
        "operators: the group Betreiber exists and lists her",
    );
    // the list is open: every asker sees it, an AI too, never Withheld
    for (who, what) in [(&q, "a human"), (&agent, "an AI account")] {
        if cx.act(who).await {
            cx.ensure(
                listed(v, &p).await,
                &format!("operators: the list is open to {what}"),
            );
        }
    }
    cx.act_me().await;
    // a mark that changes later: fail-closed read
    cx.fx.mark(&p, Kind::Agent).await;
    cx.ensure(
        !listed(v, &p).await,
        "operators: an operator later marked as an AI is not listed any more",
    );
    cx.fx.mark(&p, Kind::Unknown).await;
    cx.ensure(
        !listed(v, &p).await,
        "operators: an operator whose kind is not known is not listed",
    );
    cx.fx.mark(&p, Kind::Human).await;
    cx.ensure(
        listed(v, &p).await,
        "operators: the human mark brings her back",
    );
    cx.fx.supervise(&p, Some(true)).await;
    cx.ensure(
        !listed(v, &p).await,
        "operators: an operator who becomes supervised is not listed any more",
    );
    cx.fx.supervise(&p, None).await;
    cx.ensure(
        !listed(v, &p).await,
        "operators: an operator whose supervision is not known is not listed",
    );
    cx.fx.supervise(&p, Some(false)).await;
    cx.ensure(listed(v, &p).await, "operators: unsupervised, she is back");
    // unset
    cx.ensure(
        matches!(
            err(&local.operator_unset_local(&q).await),
            Some(Error::NotFound(_))
        ),
        "operator_unset_local: a person who is no operator is NotFound",
    );
    cx.ensure(
        local.operator_set_local(&q).await.is_ok()
            && local.operator_unset_local(&q).await.is_ok()
            && !listed(v, &q).await,
        "operator_unset_local: takes her off the list",
    );
    Some(p)
}
