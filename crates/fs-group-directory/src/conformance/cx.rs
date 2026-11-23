//! The context of a conformance run: the fixture, the report and the small helpers the rules share.
//!
//! Pattern: Test helper (plain functions; a setup that fails is a failure of the rule "setup", never a panic).

use super::fixture::{ConformanceFixture, Kind};
use crate::groups_v1::shared::{ObjectId, PersonRef, Revision};
use crate::groups_v1::{
    Error, Group, GroupArt, GroupDraft, Groups, Membership, NestOutcome, Nesting, NestingMode,
    Result, Visibility,
};

/// The failed and the skipped rules of a run; empty `failures` means conformant.
#[derive(Debug, Default)]
pub struct Report {
    /// Rules that failed.
    pub failures: Vec<String>,
    /// Rules a fixture could not set up (it has no such mutator); not a failure, but not proven either.
    pub skipped: Vec<String>,
    /// How many checks ran (a rule whose setup fails quietly is not counted; a test asserts a floor).
    pub checked: usize,
}

/// Fixture, report and a counter for names of one run.
pub(crate) struct Cx<'a> {
    pub fx: &'a dyn ConformanceFixture,
    pub g: &'a dyn Groups,
    pub report: Report,
    pub me: PersonRef,
    n: u32,
}

/// The error of a result, when it is one.
pub(crate) fn err<T>(r: &Result<T>) -> Option<&Error> {
    r.as_ref().err()
}

impl<'a> Cx<'a> {
    pub(crate) async fn new(fx: &'a dyn ConformanceFixture) -> Option<Cx<'a>> {
        let me = fx.me().await?;
        Some(Cx {
            fx,
            g: fx.groups(),
            report: Report::default(),
            me,
            n: 0,
        })
    }

    pub(crate) fn ensure(&mut self, ok: bool, rule: &str) {
        self.report.checked += 1;
        if !ok {
            self.report.failures.push(rule.to_owned());
        }
    }

    pub(crate) fn skip(&mut self, rule: &str) {
        self.report.skipped.push(rule.to_owned());
    }

    /// A draft named `Zzconf <label> <n>` (plain, everyone sees the members, not mirrored).
    pub(crate) fn draft(&mut self, label: &str) -> GroupDraft {
        self.n += 1;
        GroupDraft {
            art: GroupArt::Group,
            name: format!("Zzconf {label} {}", self.n),
            kind: None,
            description: None,
            default_mode: NestingMode::BottomUp,
            visibility: Visibility::Everyone,
            sensitive: false,
            ai: false,
            mirror: false,
        }
    }

    pub(crate) async fn group_from(&mut self, draft: GroupDraft) -> Option<Group> {
        let made = self.g.create(draft).await;
        self.ensure(
            made.is_ok(),
            &format!("setup: create group ({:?})", made.as_ref().err()),
        );
        made.ok()
    }

    pub(crate) async fn group(&mut self, label: &str) -> Option<Group> {
        let d = self.draft(label);
        self.group_from(d).await
    }

    pub(crate) async fn sensitive_group(&mut self, label: &str) -> Option<Group> {
        let mut d = self.draft(label);
        d.sensitive = true;
        self.group_from(d).await
    }

    /// A new person; a failed setup is noted.
    pub(crate) async fn person(&mut self, kind: Kind) -> Option<PersonRef> {
        let p = self.fx.person(kind).await;
        if p.is_none() {
            self.skip("setup: the fixture cannot make a person");
        }
        p
    }

    /// Say who asks; `false` (and a skip) when the fixture cannot.
    pub(crate) async fn act(&mut self, who: &PersonRef) -> bool {
        let done = self.fx.act_as(who).await;
        if !done {
            self.skip("setup: the fixture cannot change who asks");
        }
        done
    }

    pub(crate) async fn act_me(&mut self) {
        let me = self.me.clone();
        self.act(&me).await;
    }

    /// The group as stored now.
    pub(crate) async fn fresh(&self, id: ObjectId) -> Option<Group> {
        self.g.get(id).await.ok().flatten()
    }

    pub(crate) async fn rev(&self, id: ObjectId) -> Revision {
        self.fresh(id).await.map_or(Revision(0), |g| g.revision)
    }

    /// A confirmed edge made by the asking person (she manages both sides).
    pub(crate) async fn link(
        &mut self,
        child: ObjectId,
        parent: ObjectId,
        mode: NestingMode,
    ) -> Option<Nesting> {
        if let Ok(NestOutcome::Linked(e)) = self.g.request_nesting(child, parent, mode).await {
            return Some(e);
        }
        self.ensure(false, "setup: link two groups");
        None
    }

    /// A direct membership of a plain person.
    pub(crate) async fn member(
        &mut self,
        group: ObjectId,
        person: &PersonRef,
    ) -> Option<Membership> {
        let made = self.g.add_member(group, person, None).await;
        self.ensure(
            made.is_ok(),
            &format!("setup: add member ({:?})", made.as_ref().err()),
        );
        made.ok()
    }

    /// All effective members of a group.
    pub(crate) async fn effective(
        &self,
        group: ObjectId,
    ) -> Vec<crate::groups_v1::EffectiveMember> {
        self.g
            .effective_members(group, None, 1000)
            .await
            .map(|p| p.items)
            .unwrap_or_default()
    }

    /// Whether `person` is an effective member of `group`.
    pub(crate) async fn is_effective(&self, group: ObjectId, person: &PersonRef) -> bool {
        self.effective(group)
            .await
            .iter()
            .any(|m| &m.person == person)
    }
}
