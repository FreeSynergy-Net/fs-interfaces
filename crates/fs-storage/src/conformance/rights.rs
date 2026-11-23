//! Rights rules (security): a stranger reads and changes nothing, a member only reads, owners write, the trash is private.
//!
//! Pattern: Rule set. Refused calls must also leave the tree as it was.

use super::fixture::{Check, Fixture, forbidden, refused, walk, want};
use crate::ctl::Ctl;
use crate::entry::{Delete, Kind, OnConflict, PageReq, WriteOpts};
use crate::ids::EntryId;

fn opts() -> WriteOpts {
    WriteOpts {
        on_conflict: OnConflict::Fail,
        media_type: None,
    }
}

pub(super) async fn stranger_read(fx: &Fixture) -> Check {
    for (path, e) in walk(&*fx.anna).await? {
        let what = format!("stranger reads {path}");
        refused(fx.fremd.stat(&e.id).await, &what, forbidden)?;
        if e.kind == Kind::Folder {
            refused(
                fx.fremd.list(&e.id, PageReq::first(10)).await,
                &what,
                forbidden,
            )?;
        } else if e.kind == Kind::File {
            refused(fx.fremd.open(&e.id, &Ctl::none()).await, &what, forbidden)?;
        }
    }
    Ok(())
}

pub(super) async fn stranger_write(fx: &Fixture) -> Check {
    let before = walk(&*fx.anna).await?;
    for (path, e) in before.iter().filter(|(p, _)| !p.starts_with("Papierkorb")) {
        let w = format!("stranger changes {path}");
        let folder = e.kind == Kind::Folder;
        if folder {
            refused(
                fx.fremd
                    .write(&e.id, "x.txt", vec![1], opts(), &Ctl::none())
                    .await,
                &w,
                forbidden,
            )?;
            refused(
                fx.fremd
                    .create(&e.id, "x", Kind::Folder, OnConflict::Fail)
                    .await,
                &w,
                forbidden,
            )?;
        }
        refused(
            fx.fremd.rename(&e.id, "y", OnConflict::Fail).await,
            &w,
            forbidden,
        )?;
        refused(fx.fremd.delete(&e.id, Delete::Trash).await, &w, forbidden)?;
        refused(fx.fremd.delete(&e.id, Delete::Purge).await, &w, forbidden)?;
        let target = EntryId("mine-fremd".into());
        refused(
            fx.fremd.move_to(&e.id, &target, OnConflict::Fail).await,
            &w,
            forbidden,
        )?;
        refused(
            fx.fremd
                .copy_to(&e.id, &target, OnConflict::Fail, &Ctl::none())
                .await,
            &w,
            forbidden,
        )?;
    }
    want(
        walk(&*fx.anna).await? == before,
        "refused calls changed the tree",
    )
}

pub(super) async fn member_readonly(fx: &Fixture) -> Check {
    let all = walk(&*fx.anna).await?;
    let (_, file) = all
        .iter()
        .find(|(p, _)| p == "Helfa/Satzung.pdf")
        .ok_or("no Satzung.pdf")?;
    let (_, group) = all
        .iter()
        .find(|(p, _)| p == "Helfa/Protokolle")
        .ok_or("no Protokolle")?;
    want(
        super::fixture::read_all(&*fx.ben, file).await.is_ok(),
        "a member cannot read the group's file",
    )?;
    refused(
        fx.ben
            .write(&group.id, "x.txt", vec![1], opts(), &Ctl::none())
            .await,
        "member writes",
        forbidden,
    )?;
    refused(
        fx.ben.delete(&file.id, Delete::Trash).await,
        "member deletes",
        forbidden,
    )?;
    refused(
        fx.ben.rename(&file.id, "y.pdf", OnConflict::Fail).await,
        "member renames",
        forbidden,
    )?;
    let (_, mine) = all
        .iter()
        .find(|(p, _)| p == "Meine Dateien/Dokumente")
        .ok_or("no Dokumente")?;
    refused(
        fx.ben.list(&mine.id, PageReq::first(5)).await,
        "member reads another's files",
        forbidden,
    )
}

pub(super) async fn owner_write(fx: &Fixture) -> Check {
    let all = walk(&*fx.anna).await?;
    let (_, group) = all
        .iter()
        .find(|(p, _)| p == "Helfa/Protokolle")
        .ok_or("no Protokolle")?;
    let e = super::fixture::s(
        fx.anna
            .write(&group.id, "neu.txt", vec![1, 2], opts(), &Ctl::none())
            .await,
    )?;
    want(
        e.owner == group.owner,
        "a file written into a group folder must belong to the group",
    )
}

pub(super) async fn trash_private(fx: &Fixture) -> Check {
    let has = |v: &[(String, crate::Entry)]| v.iter().any(|(p, _)| p == "Papierkorb/Alt.txt");
    let mine = walk(&*fx.anna).await?;
    want(has(&mine), "the owner does not see the trashed file")?;
    want(
        !has(&walk(&*fx.fremd).await?),
        "a stranger sees another's trashed file",
    )?;
    let (_, alt) = mine
        .iter()
        .find(|(p, _)| p == "Papierkorb/Alt.txt")
        .ok_or("no Alt.txt")?;
    refused(
        fx.fremd.restore(&alt.id, None, OnConflict::Fail).await,
        "stranger restores",
        forbidden,
    )
}
