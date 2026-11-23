//! Mutants of the operator rules (user decision 16): the operator is reachable over a contract path, an AI or a
//! supervised account becomes an operator, a Project holder becomes an operator, the read side keeps a later AI.

use crate::fx::Fx;
use crate::wrap::{Mutant, Wrap};
use crate::wrap_governance::{GovMutant, WrapGov};
use async_trait::async_trait;
use fs_group_directory::FakeGroups;
use fs_group_directory::conformance::run_conformance_governance;
use fs_group_directory::groups::shared::PersonRef;
use fs_group_directory::groups::{Error, LocalOperator, Result};
use std::sync::Arc;

/// What the local entry lets through that it must refuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalMutant {
    None,
    /// An AI account is made an operator (the mark is not asked).
    AcceptsAgent,
    /// A supervised account is made an operator.
    AcceptsSupervised,
}

pub struct WrapLocal(pub Arc<FakeGroups>, pub LocalMutant);

#[async_trait]
impl LocalOperator for WrapLocal {
    async fn operator_set_local(&self, p: &PersonRef) -> Result<()> {
        let r = self.0.operator_set_local(p).await;
        let slip = matches!(
            (self.1, &r),
            (LocalMutant::AcceptsAgent, Err(Error::AgentNotAdmin))
                | (
                    LocalMutant::AcceptsSupervised,
                    Err(Error::SupervisedCannotOperate)
                )
        );
        if slip {
            self.0.insert_operator_raw(p);
            return Ok(());
        }
        r
    }
    async fn operator_unset_local(&self, p: &PersonRef) -> Result<()> {
        self.0.operator_unset_local(p).await
    }
}

/// A fake whose asker is the installer, with the given mutants in front of it.
fn fx(gov: GovMutant, base: Mutant, local: LocalMutant) -> Fx {
    let people = Arc::new(fs_group_directory::FakePeople::new());
    let me = people.human("Alex Installer");
    let fake = Arc::new(FakeGroups::with_people(people, me.clone()).with_installer(me));
    let gov = Box::new(WrapGov(Arc::clone(&fake), gov));
    let provider = Box::new(Wrap(Arc::clone(&fake), base));
    let wrapped = Arc::new(WrapLocal(Arc::clone(&fake), local));
    Fx::with(fake, provider)
        .with_governance(gov)
        .with_local(wrapped)
}

async fn failures(gov: GovMutant, base: Mutant, local: LocalMutant) -> Vec<String> {
    run_conformance_governance(&fx(gov, base, local))
        .await
        .failures
}

#[tokio::test]
async fn the_operator_rules_pass_on_the_fake_and_none_is_skipped() {
    let r = run_conformance_governance(&fx(GovMutant::None, Mutant::None, LocalMutant::None)).await;
    assert!(r.failures.is_empty(), "{:#?}", r.failures);
    assert!(r.skipped.is_empty(), "{:#?}", r.skipped);
}

#[tokio::test]
async fn an_operator_reachable_through_the_scope_fails() {
    let f = failures(
        GovMutant::ServerScopeGrantable,
        Mutant::None,
        LocalMutant::None,
    )
    .await;
    assert!(
        f.iter().any(|x| x.contains("Server is never handed out")),
        "{f:#?}"
    );
}

#[tokio::test]
async fn an_operator_reachable_through_add_member_fails() {
    let f = failures(
        GovMutant::None,
        Mutant::OperatorViaAddMember,
        LocalMutant::None,
    )
    .await;
    assert!(f.iter().any(|x| x.starts_with("add_member:")), "{f:#?}");
}

#[tokio::test]
async fn an_ai_account_as_operator_fails() {
    let f = failures(GovMutant::None, Mutant::None, LocalMutant::AcceptsAgent).await;
    assert!(
        f.iter().any(|x| x.contains("an AI account is never")),
        "{f:#?}"
    );
}

#[tokio::test]
async fn a_supervised_account_as_operator_fails() {
    let f = failures(
        GovMutant::None,
        Mutant::None,
        LocalMutant::AcceptsSupervised,
    )
    .await;
    assert!(
        f.iter()
            .any(|x| x.contains("a supervised account is never")),
        "{f:#?}"
    );
}

#[tokio::test]
async fn a_project_holder_who_becomes_operator_fails() {
    let f = failures(
        GovMutant::ProjectHolderBecomesOperator,
        Mutant::None,
        LocalMutant::None,
    )
    .await;
    assert!(
        f.iter()
            .any(|x| x.contains("a Project holder is not an operator")),
        "{f:#?}"
    );
}

#[tokio::test]
async fn a_list_that_keeps_a_later_ai_fails() {
    let f = failures(
        GovMutant::OperatorsUnfiltered,
        Mutant::None,
        LocalMutant::None,
    )
    .await;
    assert!(
        f.iter().any(|x| x.contains("later marked as an AI")),
        "{f:#?}"
    );
}

#[tokio::test]
async fn the_single_user_is_the_operator() {
    let people = Arc::new(fs_group_directory::FakePeople::new());
    let me = people.human("Alex Solo");
    let fake = FakeGroups::with_people(people, me.clone()).with_operator(&me);
    let list = fs_group_directory::groups::GroupGovernance::operators(&fake)
        .await
        .expect("the list is open");
    assert_eq!(list.persons, vec![me]);
    assert!(list.group.is_some());
}

#[test]
fn only_the_scope_server_covers_server() {
    use fs_group_directory::groups::Scope;
    assert!(!Scope::Project.covers(Scope::Server));
    assert!(!Scope::AllGroups.covers(Scope::Server));
    assert!(Scope::Server.covers(Scope::Server));
    assert!(!Scope::Server.covers(Scope::Project));
}
