//! The mirror runs: `mirror_plan`, `mirror_apply`, `mirror_status`, `group_mirror_state` of the fake.
//!
//! Pattern: Template method + Memento (plan, check the ticket against a fresh plan, write step by step in order,
//! store the report; a changed world makes the ticket stale).

use crate::groups_v1::shared::ObjectId;
use crate::groups_v1::{
    ChangeKind, DirectoryGroup, DirectoryId, Error, FindingId, GroupMirrorState, MirrorError,
    MirrorFinding, MirrorPlan, MirrorReport, MirrorStatus, MirrorStep, MirrorTicket, Result, Right,
};
use crate::ids::object_id;
use crate::mirror_state::{Run, StoredPlan};
use crate::state::State;

pub(crate) type MirrorResult<T> = std::result::Result<T, MirrorError>;

impl State {
    /// An asking AI may not read the mirror at all; the id in the error is no group's.
    pub(crate) fn mirror_gate(&self) -> Result<()> {
        if self.asker_is_agent() {
            Err(Error::Forbidden {
                group: object_id(0),
                right: Right::Read,
            })
        } else {
            Ok(())
        }
    }

    fn known_nests(&self, directory: &DirectoryId) -> Vec<(ObjectId, ObjectId)> {
        self.mirror
            .last(directory)
            .map_or_default(|r| r.nests.clone())
    }

    pub(crate) fn mirror_plan(
        &mut self,
        directory: DirectoryId,
        current: &[DirectoryGroup],
    ) -> MirrorResult<MirrorPlan> {
        self.mirror_gate().map_err(MirrorError::Groups)?;
        let made = self.tick();
        let c = self.compute_plan(current, &self.known_nests(&directory));
        self.mirror.tickets += 1;
        let ticket = self.mirror.tickets;
        self.mirror.plans.push(StoredPlan {
            ticket,
            directory: directory.clone(),
            steps: c.steps.clone(),
            findings: c.findings.clone(),
        });
        Ok(MirrorPlan {
            ticket: MirrorTicket(ticket),
            directory,
            made,
            steps: c.steps,
            findings: c.findings,
        })
    }

    /// The plan of `ticket` when it is still exactly what a fresh plan says.
    pub(crate) fn mirror_check(
        &self,
        directory: &DirectoryId,
        ticket: &MirrorTicket,
        overwrite: &[FindingId],
        current: &[DirectoryGroup],
    ) -> MirrorResult<StoredPlan> {
        self.mirror_gate().map_err(MirrorError::Groups)?;
        let stored = self
            .mirror
            .plans
            .iter()
            .find(|p| p.ticket == ticket.0 && &p.directory == directory)
            .cloned();
        let stored = stored.ok_or(MirrorError::PlanStale)?;
        if overwrite
            .iter()
            .any(|o| !stored.findings.iter().any(|f| f.id == *o && f.overwritable))
        {
            return Err(MirrorError::PlanStale);
        }
        let fresh = self.compute_plan(current, &self.known_nests(directory));
        if fresh.steps == stored.steps && fresh.findings == stored.findings {
            Ok(stored)
        } else {
            Err(MirrorError::PlanStale)
        }
    }

    /// Store a finished run and log the changed groups.
    pub(crate) fn mirror_store(&mut self, run: Run) -> MirrorReport {
        let at = self.tick();
        let mut report = run.report.clone();
        report.ended = at;
        for s in &report.applied {
            let (MirrorStep::Create { group }
            | MirrorStep::Update { group, .. }
            | MirrorStep::Remove { group }) = s;
            self.change(ChangeKind::Mirror { group: *group });
        }
        self.mirror.runs.push(Run {
            report: report.clone(),
            ..run
        });
        report
    }

    pub(crate) fn mirror_status(&self) -> Result<Vec<MirrorStatus>> {
        self.mirror_gate()?;
        let mut out: Vec<MirrorStatus> = Vec::new();
        for run in self.mirror.runs.iter().rev() {
            if out.iter().any(|s| s.directory == run.report.directory) {
                continue;
            }
            let count = u32::try_from(run.mirrored.len()).unwrap_or(u32::MAX);
            out.push(MirrorStatus {
                directory: run.report.directory.clone(),
                last: Some(run.report.clone()),
                groups_mirrored: count,
            });
        }
        Ok(out)
    }

    pub(crate) fn group_mirror_state(
        &self,
        group: ObjectId,
    ) -> Result<Vec<(DirectoryId, GroupMirrorState)>> {
        self.mirror_gate()?;
        let g = self.visible(group)?;
        let off = !g.mirror || g.sensitive;
        let mut out: Vec<(DirectoryId, GroupMirrorState)> = Vec::new();
        for run in self.mirror.runs.iter().rev() {
            let dir = run.report.directory.clone();
            if out.iter().any(|(d, _)| *d == dir) {
                continue;
            }
            let findings: Vec<MirrorFinding> = run
                .report
                .findings
                .iter()
                .filter(|f| f.group == Some(group))
                .cloned()
                .collect();
            let at = run.report.ended;
            let state = if off {
                GroupMirrorState::NotMirrored
            } else if run.held.contains(&group) {
                GroupMirrorState::Failed { at, findings }
            } else if run.mirrored.contains(&group) {
                GroupMirrorState::Synced { at, findings }
            } else {
                GroupMirrorState::Pending
            };
            out.push((dir, state));
        }
        Ok(out)
    }
}
