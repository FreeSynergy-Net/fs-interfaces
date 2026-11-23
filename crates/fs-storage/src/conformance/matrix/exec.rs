//! The driver: one cell = one fresh world, one call by one actor, compared with the expected code; refused calls must
//! leave the tree as it was (G1), allowed changes must change it.
//!
//! Pattern: Interpreter of the table.

use super::model::{ACTORS, Actor, Expect, Op, Row, Target};
use super::setup::{Setup, build, provider};
use crate::conformance::fixture::{Check, Fixture, Report, walk};
use crate::ctl::{Ctl, MemSource};
use crate::entry::{Delete, Kind, OnConflict, PageReq, WriteOpts};
use crate::error::Result;
use crate::ids::EntryId;

fn folder_op(op: Op) -> bool {
    matches!(
        op,
        Op::List
            | Op::Write
            | Op::WriteCancelled
            | Op::Create
            | Op::CreateFile
            | Op::MoveDst
            | Op::CopyDst
            | Op::RestoreDst
    )
}

fn changes_state(op: Op) -> bool {
    !matches!(op, Op::Stat | Op::List | Op::Open)
}

/// The call of `op` by `a` on `id` (the target); `None`: nothing to call.
#[allow(clippy::many_single_char_names)]
async fn call(
    fx: &Fixture,
    su: &Setup,
    a: Actor,
    op: Op,
    t: Target,
    id: &EntryId,
) -> Option<Result<()>> {
    let p = if t == Target::T13 {
        &*fx.local
    } else {
        provider(fx, a)
    };
    let own = su.of(a);
    let (own_folder, own_file) = match (t, own) {
        (Target::T13, _) => (su.local_root.clone(), su.local_file.clone()),
        (_, Some(o)) if matches!(op, Op::MoveDst | Op::CopyDst) => {
            (o.folder.clone(), o.file2.clone())
        }
        (_, Some(o)) => (o.folder.clone(), o.file.clone()),
        (_, None) => (su.gfolder.clone(), su.gfile.clone()),
    };
    let o = || WriteOpts {
        generator: None,
        on_conflict: OnConflict::Fail,
        media_type: None,
    };
    let body = || MemSource::from(vec![1_u8]);
    let ctl = Ctl::none();
    let r = match op {
        Op::Stat => p.stat(id).await.map(|_| ()),
        Op::List => p.list(id, PageReq::first(10)).await.map(|_| ()),
        Op::Open => p.open(id, &ctl).await.map(|_| ()),
        Op::Write => p
            .write(id, "neu.txt", &mut body(), o(), &ctl)
            .await
            .map(|_| ()),
        Op::WriteCancelled => {
            let c = Ctl::none();
            c.cancel();
            p.write(id, "neu.txt", &mut body(), o(), &c)
                .await
                .map(|_| ())
        }
        Op::Create => p
            .create(id, "neu-ordner", Kind::Folder, OnConflict::Fail)
            .await
            .map(|_| ()),
        Op::CreateFile => p
            .create(id, "neu-datei", Kind::File, OnConflict::Fail)
            .await
            .map(|_| ()),
        Op::Rename => p
            .rename(id, "neuer-name", OnConflict::Fail)
            .await
            .map(|_| ()),
        Op::MoveSrc => p
            .move_to(id, &own_folder, OnConflict::Fail)
            .await
            .map(|_| ()),
        Op::MoveDst => p.move_to(&own_file, id, OnConflict::Fail).await.map(|_| ()),
        Op::CopySrc => p
            .copy_to(id, &own_folder, OnConflict::Fail, &ctl)
            .await
            .map(|_| ()),
        Op::CopyDst => p
            .copy_to(&own_file, id, OnConflict::Fail, &ctl)
            .await
            .map(|_| ()),
        Op::MoveCycle | Op::CopyCycle => {
            let o = own?;
            if op == Op::MoveCycle {
                p.move_to(&o.folder, &o.sub, OnConflict::Fail)
                    .await
                    .map(|_| ())
            } else {
                p.copy_to(&o.folder, &o.sub, OnConflict::Fail, &ctl)
                    .await
                    .map(|_| ())
            }
        }
        Op::Trash => p.delete(id, Delete::Trash).await,
        Op::Purge => p.delete(id, Delete::Purge).await,
        Op::RestoreSrc => p.restore(id, None, OnConflict::Fail).await.map(|_| ()),
        Op::RestoreDst => {
            let src = match (t, own) {
                (Target::T13, _) => su.local_file.clone(),
                (_, Some(o)) => o.tfile.clone(),
                (_, None) => su.trashed_gfile.clone(),
            };
            p.restore(&src, Some(id), OnConflict::Fail)
                .await
                .map(|_| ())
        }
    };
    Some(r)
}

async fn cell(make: &dyn Fn() -> Fixture, row: &Row, a: Actor, want: Expect) -> Check {
    if want == Expect::Na {
        return Ok(());
    }
    let fx = make();
    let su = build(&fx).await?;
    let folder = folder_op(row.op);
    let id = if matches!(row.op, Op::MoveCycle | Op::CopyCycle) {
        su.target(a, Target::T1, true)
    } else {
        su.target(a, row.target, folder)
    };
    let Some(id) = id else {
        return Err(format!("{a:?} has no target {:?}", row.target));
    };
    let before = walk(&*fx.operator).await?;
    let Some(r) = call(&fx, &su, a, row.op, row.target, &id).await else {
        return Ok(());
    };
    let after = walk(&*fx.operator).await?;
    if !want.holds(&r) {
        return Err(format!(
            "{a:?}: expected {want:?}, got {}",
            r.as_ref()
                .map_or_else(ToString::to_string, |()| "Ok".to_owned())
        ));
    }
    if r.is_err() && before != after {
        return Err(format!("{a:?}: a refused call changed the tree (G1)"));
    }
    if want == Expect::Ok && changes_state(row.op) && before == after {
        return Err(format!("{a:?}: an allowed change left no trace"));
    }
    Ok(())
}

/// Run every row for every actor; one report entry per row (a read-only row runs once: all columns are equal).
pub(in crate::conformance) async fn run_rows(
    report: &mut Report,
    make: &dyn Fn() -> Fixture,
    rows: &[Row],
) {
    for row in rows {
        let cells = row.parsed();
        let mut verdict: Check = Ok(());
        for (a, (want, _)) in ACTORS.into_iter().zip(cells) {
            if row.target == Target::T13 && a != Actor::E {
                continue;
            }
            if let Err(why) = cell(make, row, a, want).await {
                verdict = Err(why);
                break;
            }
        }
        report.record(row.rule(), verdict);
    }
}
