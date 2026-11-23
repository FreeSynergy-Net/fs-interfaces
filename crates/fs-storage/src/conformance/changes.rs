//! "Nobody but the owners changes foreign content": every change call on a foreign entry must be Forbidden and leave the
//! tree as it was. Used for the manager (admin level (a)/(b)) and for the operator (decision 16.3: changing or deleting
//! foreign files only through an intervention with a reason, which is not part of this contract).
//!
//! Pattern: Rule set helper (one table of change calls, run against every foreign entry).

use super::fixture::{Check, forbidden, refused, walk, want};
use crate::ctl::{Ctl, MemSource};
use crate::entry::{Delete, Kind, OnConflict, WriteOpts};
use crate::ids::EntryId;
use crate::provider::StorageProvider;

/// `actor` runs every change call on every entry of Anna's tree (private, group, trash). `anna` shows the tree before and
/// after. `skip_private` is for a manager, who cannot even see private entries (Forbidden anyway, still checked).
pub(super) async fn none_allowed(
    actor: &dyn StorageProvider,
    anna: &dyn StorageProvider,
    who: &str,
) -> Check {
    let before = walk(anna).await?;
    let foreign: Vec<_> = before
        .iter()
        .filter(|(_, e)| e.id.as_str() != "l-helfa")
        .collect();
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
    )
}
