//! `run`: every rule, each on a fresh fixture.
//!
//! Pattern: Template method.

use super::fixture::{Fixture, Report};
use super::{admin, names, operator_log, read, rights, scope, write};

/// Run all rules. `make` builds a fresh fixture (a fresh sample world) for every rule, so no rule sees the changes of
/// another.
pub async fn run(make: &dyn Fn() -> Fixture) -> Report {
    let mut r = Report::default();
    r.record("roots.shape", read::roots_shape(&make()).await);
    r.record("list.paging", read::paging(&make()).await);
    r.record("tree.files", read::tree_files(&make()).await);
    r.record("file.sha256", read::file_sha256(&make()).await);
    r.record("blob.range", read::blob_range(&make()).await);
    r.record("scope.generator", scope::scope_generator(&make()).await);
    r.record("owner.kinds", read::owner_kinds(&make()).await);
    r.record("stat.matches-list", read::stat_matches(&make()).await);
    r.record("caps.honest", read::caps_honest(&make()).await);
    r.record("rights.stranger.read", rights::stranger_read(&make()).await);
    r.record(
        "rights.stranger.write",
        rights::stranger_write(&make()).await,
    );
    r.record(
        "rights.member.readonly",
        rights::member_readonly(&make()).await,
    );
    r.record("rights.owner.write", rights::owner_write(&make()).await);
    r.record("rights.trash.private", rights::trash_private(&make()).await);
    r.record("rights.manager.no-server", admin::admin(&make()).await);
    r.record("rights.operator.reads-all", admin::operator(&make()).await);
    r.record("rights.bot", admin::bot(&make()).await);
    r.record("local.import-only", admin::local_import_only(&make()).await);
    r.record("names.bad", names::bad_names(&make()).await);
    r.record("path.traversal", names::traversal(&make()).await);
    r.record("link.never-followed", names::links(&make()).await);
    r.record(
        "conflict.never-replace",
        write::never_replace(&make()).await,
    );
    r.record("write.roundtrip", write::roundtrip(&make()).await);
    r.record("trash.restore-purge", write::trash_restore(&make()).await);
    r.record("move.cycle-copy", write::move_copy(&make()).await);
    r.record("changes.delta", write::delta(&make()).await);
    r.record("write.cancel", write::cancel(&make()).await);
    r.record("conflict.rename", write::rename_conflict(&make()).await);
    r.record("operator.log", operator_log::operator_log(&make()).await);
    super::matrix::run_rows(&mut r, make, &super::matrix::rows()).await;
    r
}
