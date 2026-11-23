//! The async part of a mirror run: the steps against the directory, the overwrite of confirmed findings, and the
//! report.
//!
//! Pattern: Template method (write step by step in order; a failed step ends the run as `Partial` or `Failed`).

use crate::groups::shared::{ObjectId, PersonRef};
use crate::groups::{
    DirectoryError, DirectoryGroup, FindingId, FindingKind, GroupDirectory, MirrorFinding,
    MirrorReport, MirrorStep, RunOutcome,
};
use crate::mirror_apply::MirrorResult;
use crate::mirror_state::{Run, StoredPlan};
use crate::state::State;

/// Apply one step to the directory; the persons without an account come back.
async fn do_step(
    dir: &dyn GroupDirectory,
    step: &MirrorStep,
    names: &std::collections::BTreeMap<ObjectId, String>,
) -> std::result::Result<Vec<PersonRef>, DirectoryError> {
    match step {
        MirrorStep::Create { group } => {
            let blank = DirectoryGroup {
                group: *group,
                name: names.get(group).cloned().unwrap_or_default(),
                members: Vec::new(),
                nested: Vec::new(),
                foreign: Vec::new(),
            };
            dir.apply(&blank).await.map(|a| a.unmapped)
        }
        MirrorStep::Remove { group } => dir.remove(*group).await.map(|()| Vec::new()),
        MirrorStep::Update {
            group,
            add,
            remove,
            nest,
            unnest,
        } => {
            let mut g = dir.read(*group).await?.unwrap_or(DirectoryGroup {
                group: *group,
                name: names.get(group).cloned().unwrap_or_default(),
                members: Vec::new(),
                nested: Vec::new(),
                foreign: Vec::new(),
            });
            g.members.retain(|p| !remove.contains(p));
            let fresh: Vec<PersonRef> = add
                .iter()
                .filter(|p| !g.members.contains(p))
                .cloned()
                .collect();
            g.members.extend(fresh);
            g.nested.retain(|n| !unnest.contains(n));
            let fresh: Vec<ObjectId> = nest
                .iter()
                .filter(|n| !g.nested.contains(n))
                .copied()
                .collect();
            g.nested.extend(fresh);
            dir.apply(&g).await.map(|a| a.unmapped)
        }
    }
}

/// The by-hand things a human confirmed to overwrite.
async fn overwrite_one(
    dir: &dyn GroupDirectory,
    f: &MirrorFinding,
) -> std::result::Result<(), DirectoryError> {
    let (Some(group), kind) = (f.group, &f.kind) else {
        return Ok(());
    };
    let Some(mut g) = dir.read(group).await? else {
        return Ok(());
    };
    match kind {
        FindingKind::Foreign(account) => g.foreign.retain(|a| a != account),
        FindingKind::ForeignNesting { inner, .. } => g.nested.retain(|n| n != inner),
        _ => return Ok(()),
    }
    dir.apply(&g).await.map(|_| ())
}

impl State {
    /// Run the checked plan against the directory (async part outside the lock is the caller's: this takes the
    /// results of the directory calls and stores the report).
    pub(crate) async fn mirror_run(
        cell: &std::sync::Mutex<Self>,
        dir: &dyn GroupDirectory,
        plan: StoredPlan,
        overwrite: &[FindingId],
    ) -> MirrorResult<MirrorReport> {
        let (names, started, actor): (std::collections::BTreeMap<ObjectId, String>, _, _) = {
            let s = cell
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            (
                s.groups
                    .iter()
                    .map(|(i, r)| (*i, r.group.name.clone()))
                    .collect(),
                s.now,
                s.asker.clone(),
            )
        };
        let (mut applied, mut unmapped, mut failed) = (Vec::new(), Vec::new(), false);
        for step in &plan.steps {
            if let Ok(u) = do_step(dir, step, &names).await {
                unmapped.extend(u);
                applied.push(step.clone());
            } else {
                failed = true;
                break;
            }
        }
        let mut findings: Vec<MirrorFinding> = Vec::new();
        for f in &plan.findings {
            let go = f.overwritable && overwrite.contains(&f.id) && !failed;
            if !go || overwrite_one(dir, f).await.is_err() {
                findings.push(f.clone());
            }
        }
        findings.extend(unmapped.into_iter().map(|p| MirrorFinding {
            id: FindingId(0),
            group: None,
            kind: FindingKind::Unmapped(p),
            overwritable: false,
        }));
        let outcome = match (failed, applied.is_empty(), findings.is_empty()) {
            (true, true, _) => RunOutcome::Failed,
            (true, false, _) => RunOutcome::Partial,
            (false, _, true) => RunOutcome::Done,
            (false, _, false) => RunOutcome::DoneWithFindings,
        };
        let current: Vec<DirectoryGroup> = dir.list().await.unwrap_or_default();
        let mut s = cell
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let held: Vec<ObjectId> = findings
            .iter()
            .filter_map(|f| {
                if let FindingKind::HeldBack(g) = f.kind {
                    Some(g)
                } else {
                    None
                }
            })
            .collect();
        let c = s.compute_plan(&current, &[]);
        let mirrored: Vec<ObjectId> = if failed {
            Vec::new()
        } else {
            c.wanted
                .keys()
                .filter(|g| !held.contains(g))
                .copied()
                .collect()
        };
        let nests = c
            .wanted
            .values()
            .flat_map(|w| w.nested.iter().map(|n| (w.group, *n)))
            .collect();
        let directory = plan.directory.clone();
        let report = MirrorReport {
            directory,
            started,
            ended: started,
            outcome,
            applied,
            findings,
            actor,
        };
        Ok(s.mirror_store(Run {
            report,
            mirrored,
            held,
            nests,
        }))
    }
}
