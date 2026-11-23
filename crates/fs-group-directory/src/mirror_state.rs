//! What the fake keeps of the mirror runs: stored plans (tickets) and finished runs per directory.
//!
//! Pattern: Memento (a ticket binds `mirror_apply` to the plan that was shown; a run keeps what was mirrored).

use crate::groups::shared::ObjectId;
use crate::groups::{DirectoryId, MirrorFinding, MirrorReport, MirrorStep};

/// A plan that was shown.
#[derive(Clone, Debug)]
pub(crate) struct StoredPlan {
    pub ticket: u64,
    pub directory: DirectoryId,
    pub steps: Vec<MirrorStep>,
    pub findings: Vec<MirrorFinding>,
}

/// A finished run.
#[derive(Clone, Debug)]
pub(crate) struct Run {
    pub report: MirrorReport,
    /// Groups that are in sync after the run.
    pub mirrored: Vec<ObjectId>,
    /// Groups the run held back.
    pub held: Vec<ObjectId>,
    /// The arcs (outer, inner) fs-groups wrote, so a later removal is an `unnest` and not a foreign arc.
    pub nests: Vec<(ObjectId, ObjectId)>,
}

/// Plans and runs of one fake.
#[derive(Debug, Default)]
pub(crate) struct MirrorLog {
    pub tickets: u64,
    pub plans: Vec<StoredPlan>,
    pub runs: Vec<Run>,
}

impl MirrorLog {
    /// The last run in a directory.
    pub(crate) fn last(&self, directory: &DirectoryId) -> Option<&Run> {
        self.runs
            .iter()
            .rev()
            .find(|r| &r.report.directory == directory)
    }
}
