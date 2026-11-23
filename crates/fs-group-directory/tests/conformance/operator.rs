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
    /// An account whose supervision is not known is made an operator (unknown is read as "not supervised").
    AcceptsUnknownSupervision,
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
        ) || (self.1 == LocalMutant::AcceptsUnknownSupervision
            && matches!(r, Err(Error::SupervisedCannotOperate))
            && self.0.people().supervised(p).is_none());
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
    assert!(r.is_complete(), "{r:#?}");
}

/// A provider that offers no `LocalOperator` (like `RemoteGroups`): the contract rules still run.
async fn without_local(gov: GovMutant, base: Mutant) -> fs_group_directory::conformance::Report {
    run_conformance_governance(&fx_no_local(gov, base)).await
}

fn fx_no_local(gov: GovMutant, base: Mutant) -> Fx {
    fx(gov, base, LocalMutant::None).without_local()
}

#[tokio::test]
async fn a_provider_without_a_local_entry_is_not_fully_conformant_but_not_red() {
    let r = without_local(GovMutant::None, Mutant::None).await;
    assert!(r.failures.is_empty(), "{:#?}", r.failures);
    assert!(!r.unverifiable.is_empty() && !r.is_complete(), "{r:#?}");
}

#[tokio::test]
async fn holes_in_a_provider_without_a_local_entry_are_found() {
    let r = without_local(
        GovMutant::ServerScopeGrantable,
        Mutant::OperatorViaAddMember,
    )
    .await;
    assert!(
        r.failures
            .iter()
            .any(|x| x.contains("Server is never handed out")),
        "{r:#?}"
    );
    assert!(
        r.failures.iter().any(|x| x.starts_with("add_member:")),
        "{r:#?}"
    );
    assert!(!r.is_complete());
}

#[tokio::test]
async fn an_account_of_unknown_supervision_as_operator_fails() {
    let f = failures(
        GovMutant::None,
        Mutant::None,
        LocalMutant::AcceptsUnknownSupervision,
    )
    .await;
    assert!(
        f.iter()
            .any(|x| x.contains("supervision is not known is never")),
        "{f:#?}"
    );
}

#[tokio::test]
async fn a_group_betreiber_that_gets_a_scope_fails() {
    let f = failures(
        GovMutant::OperatorGroupGetsScope,
        Mutant::None,
        LocalMutant::None,
    )
    .await;
    assert!(
        f.iter().any(|x| x.contains("holds no scope but Server")),
        "{f:#?}"
    );
}

#[tokio::test]
async fn a_group_betreiber_that_holds_a_right_fails() {
    let f = failures(
        GovMutant::None,
        Mutant::OperatorGroupHoldsGrant,
        LocalMutant::None,
    )
    .await;
    assert!(
        f.iter().any(|x| x.contains("holds no right anywhere")),
        "{f:#?}"
    );
}

#[tokio::test]
async fn a_provider_that_gives_the_reserved_name_away_fails() {
    let f = failures(GovMutant::None, Mutant::ReservedNameFree, LocalMutant::None).await;
    assert!(
        f.iter()
            .any(|x| x.contains("is reserved for the group Betreiber")),
        "{f:#?}"
    );
}

#[test]
fn the_reserved_name_is_found_in_its_spellings() {
    use fs_group_directory::groups::is_reserved_group_name as r;
    for yes in [
        "Betreiber",
        " betreiber ",
        "BETREIBER",
        "Be-treiber",
        "B.e t_reiber",
        "Be\u{200B}treiber",
        "\u{FF22}\u{FF25}\u{FF34}\u{FF32}\u{FF25}\u{FF29}\u{FF22}\u{FF25}\u{FF32}",
        "Operators",
        "Betreiberinnen",
    ] {
        assert!(r(yes), "{yes:?}");
    }
    for no in [
        "Betreiber Hilfe",
        "Betreiber 2",
        "Vereinsbetreiber",
        "Zzconf",
        "",
    ] {
        assert!(!r(no), "{no:?}");
    }
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
