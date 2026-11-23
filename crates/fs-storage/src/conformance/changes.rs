//! "Nobody but the owners changes foreign content": every change call on a foreign entry must be Forbidden and leave the
//! tree as it was. Used for the manager (admin level (a)/(b)) and for the operator (decision 16.3: changing or deleting
//! foreign files only through an intervention with a reason, which is not part of this contract).
//!
//! Pattern: Rule set helper (one table of change calls, run against every foreign entry).

use super::fixture::{Check, forbidden, refused, walk, want};
use crate::ctl::{Ctl, MemSource};
use crate::entry::{Delete, Kind, OnConflict, RootKind, WriteOpts};
use crate::ids::EntryId;
use crate::provider::StorageProvider;

/// `actor` runs every change call on every entry of Anna's tree (the root folders, private, group, links, trash). `anna`
/// shows the tree before and after. Rights come before validity: a refused call is `Forbidden` whatever else is wrong.
/// Afterwards the actor's OWN file is moved, copied and restored into every foreign folder (see `own_into_foreign`).
pub(super) async fn none_allowed(
    actor: &dyn StorageProvider,
    anna: &dyn StorageProvider,
    who: &str,
) -> Check {
    let before = walk(anna).await?;
    let mut roots = Vec::new();
    for r in anna.roots().await.map_err(|e| e.to_string())? {
        let own_trash = actor
            .roots()
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|t| t.kind == RootKind::Trash);
        if r.kind != RootKind::Trash || own_trash.is_none_or(|t| t.id != r.id) {
            let e = anna.stat(&r.id).await.map_err(|e| e.to_string())?;
            roots.push((format!("root {}", r.name), e));
        }
    }
    let foreign: Vec<_> = roots.iter().chain(before.iter()).collect();
    let target = foreign
        .iter()
        .find(|(p, e)| p == "Meine Dateien/Dokumente" && e.kind == Kind::Folder)
        .map(|(_, e)| e.id.clone())
        .ok_or("no foreign target folder")?;
    let group = foreign
        .iter()
        .find(|(_, e)| e.id.as_str() == "grp-helfa")
        .map_or_else(|| EntryId("grp-helfa".into()), |(_, e)| e.id.clone());
    let o = || WriteOpts {
        generator: None,
        on_conflict: OnConflict::Fail,
        media_type: None,
    };
    for (path, e) in &foreign {
        let w = format!("{who} changes {path}");
        refused(
            actor.rename(&e.id, "geaendert", OnConflict::Fail).await,
            &w,
            forbidden,
        )?;
        refused(actor.delete(&e.id, Delete::Trash).await, &w, forbidden)?;
        refused(actor.delete(&e.id, Delete::Purge).await, &w, forbidden)?;
        refused(
            actor.restore(&e.id, None, OnConflict::Fail).await,
            &w,
            forbidden,
        )?;
        refused(
            actor.move_to(&e.id, &group, OnConflict::KeepBoth).await,
            &w,
            forbidden,
        )?;
        refused(
            actor
                .copy_to(&e.id, &target, OnConflict::KeepBoth, &Ctl::none())
                .await,
            &w,
            forbidden,
        )?;
        if e.kind == Kind::Folder {
            refused(
                actor
                    .write(
                        &e.id,
                        "x.txt",
                        &mut MemSource::from(vec![1]),
                        o(),
                        &Ctl::none(),
                    )
                    .await,
                &w,
                forbidden,
            )?;
            refused(
                actor
                    .create(&e.id, "neu", Kind::Folder, OnConflict::Fail)
                    .await,
                &w,
                forbidden,
            )?;
        }
    }
    want(
        walk(anna).await? == before,
        &format!("{who}: refused changes altered the tree"),
    )?;
    own_into_foreign(actor, anna, who).await
}

/// The actor writes a file into its OWN root and then tries to put it into every foreign folder, the root folders
/// included: `move_to`, `copy_to`, and `restore` of the trashed file with a foreign target. Each must be `Forbidden`
/// (the right on the TARGET counts, not only on the source) and Anna's tree stays as it was. The own root is the "My
/// files" whose owner is the owner of the actor's trash root. A provider without such a root (a bot) has nothing to try.
pub(super) async fn own_into_foreign(
    actor: &dyn StorageProvider,
    anna: &dyn StorageProvider,
    who: &str,
) -> Check {
    let roots = actor.roots().await.map_err(|e| e.to_string())?;
    let Some(trash) = roots.iter().find(|r| r.kind == RootKind::Trash) else {
        return Ok(());
    };
    let Some(own) = roots
        .iter()
        .find(|r| r.kind == RootKind::Mine && r.owner.id == trash.owner.id)
    else {
        return Ok(());
    };
    let o = WriteOpts {
        generator: None,
        on_conflict: OnConflict::KeepBoth,
        media_type: None,
    };
    let mine = actor
        .write(
            &own.id,
            "eigen.txt",
            &mut MemSource::from(vec![7]),
            o,
            &Ctl::none(),
        )
        .await
        .map_err(|e| format!("{who} cannot write into its own root: {e}"))?;
    let before = walk(anna).await?;
    let mut targets: Vec<(String, EntryId)> = Vec::new();
    for r in anna.roots().await.map_err(|e| e.to_string())? {
        targets.push((format!("root {}", r.name), r.id));
    }
    targets.extend(
        before
            .iter()
            .filter(|(_, e)| e.kind != Kind::File)
            .map(|(p, e)| (p.clone(), e.id.clone())),
    );
    targets.retain(|(_, id)| id != &own.id && id.as_str() != trash.id.as_str());
    for (path, target) in &targets {
        let w = format!("{who} puts its own file into {path}");
        refused(
            actor.move_to(&mine.id, target, OnConflict::KeepBoth).await,
            &w,
            forbidden,
        )?;
        refused(
            actor
                .copy_to(&mine.id, target, OnConflict::KeepBoth, &Ctl::none())
                .await,
            &w,
            forbidden,
        )?;
        actor
            .delete(&mine.id, Delete::Trash)
            .await
            .map_err(|e| e.to_string())?;
        refused(
            actor
                .restore(&mine.id, Some(target), OnConflict::KeepBoth)
                .await,
            &w,
            forbidden,
        )?;
        actor
            .restore(&mine.id, None, OnConflict::KeepBoth)
            .await
            .map_err(|e| format!("{who} cannot restore to its own root: {e}"))?;
    }
    want(
        walk(anna).await? == before,
        &format!("{who}: own file in a foreign folder altered the tree"),
    )
}
