//! Admin, bot and foreign-tree rules (security): an admin sees structure, not private content; a bot has nothing; `local`
//! is no place to save to.
//!
//! Pattern: Rule set. User decisions of 23.11.2026 (files addendum): private folders show the admin only `usage`;
//! group folders show the structure, the content needs an intervention with a reason (ADR 0095 point 7, not part of this
//! contract yet: here `open` without it must be Forbidden).

use super::fixture::{Check, Fixture, all, forbidden, refused, root_of, s, walk, want};
use crate::ctl::Ctl;
use crate::entry::{Delete, Kind, OnConflict, PageReq, RootKind, WriteOpts};
use crate::error::Error;

fn unsupported(e: &Error) -> bool {
    matches!(e, Error::Unsupported { .. })
}

pub(super) async fn admin(fx: &Fixture) -> Check {
    let p = &*fx.admin;
    let mine = root_of(&*fx.anna, RootKind::Mine).await?;
    let group = root_of(&*fx.anna, RootKind::Group).await?;
    let roots = s(p.roots().await)?;
    let u = roots
        .iter()
        .find(|r| r.id == mine)
        .and_then(|r| r.usage)
        .ok_or("the admin gets no usage of a private folder")?;
    want(
        u.items > 0 && u.bytes > 0,
        "usage of a private folder is empty",
    )?;
    refused(
        p.list(&mine, PageReq::first(10)).await,
        "admin lists a private folder",
        forbidden,
    )?;
    refused(
        p.stat(&mine).await,
        "admin stats a private folder",
        forbidden,
    )?;
    for (path, e) in walk(&*fx.anna)
        .await?
        .iter()
        .filter(|(p, _)| p.starts_with("Meine Dateien/"))
    {
        refused(
            p.stat(&e.id).await,
            &format!("admin stats {path}"),
            forbidden,
        )?;
        if e.kind == Kind::File {
            refused(
                p.open(&e.id, &Ctl::none()).await,
                &format!("admin opens {path}"),
                forbidden,
            )?;
        }
    }
    want(
        roots.iter().any(|r| r.id == group),
        "the admin does not see the group folder",
    )?;
    want(
        all(p, &group).await? == all(&*fx.anna, &group).await?,
        "the admin does not see the group structure",
    )?;
    for (path, e) in walk(&*fx.anna)
        .await?
        .iter()
        .filter(|(p, e)| p.starts_with("Helfa/") && e.kind == Kind::File)
    {
        s(p.stat(&e.id).await)?;
        refused(
            p.open(&e.id, &Ctl::none()).await,
            &format!("admin opens {path} without an intervention"),
            forbidden,
        )?;
    }
    Ok(())
}

pub(super) async fn bot(fx: &Fixture) -> Check {
    let p = &*fx.bot;
    let roots = s(p.roots().await)?;
    want(
        roots
            .iter()
            .all(|r| r.kind != RootKind::Mine && r.kind != RootKind::Group),
        "a bot gets a folder",
    )?;
    let all_entries = walk(&*fx.anna).await?;
    want(
        all_entries.iter().all(|(_, e)| e.owner != fx.bot_ref),
        "a bot owns an entry",
    )?;
    for (path, e) in all_entries.iter().filter(|(p, _)| p.starts_with("Helfa")) {
        refused(p.stat(&e.id).await, &format!("bot stats {path}"), forbidden)?;
    }
    let group = root_of(&*fx.anna, RootKind::Group).await?;
    let o = WriteOpts {
        on_conflict: OnConflict::Fail,
        media_type: None,
    };
    refused(
        p.write(&group, "bot.txt", vec![1], o, &Ctl::none()).await,
        "bot writes",
        forbidden,
    )?;
    refused(
        p.create(&group, "bot", Kind::Folder, OnConflict::Fail)
            .await,
        "bot creates",
        forbidden,
    )
}

pub(super) async fn local_import_only(fx: &Fixture) -> Check {
    let p = &*fx.local;
    want(!p.capabilities().write, "a local tree says it can write")?;
    let roots = s(p.roots().await)?;
    let root = roots
        .iter()
        .find(|r| r.kind == RootKind::External)
        .ok_or("no External root")?;
    let items = all(p, &root.id).await?;
    let file = items
        .iter()
        .find(|e| e.kind == Kind::File)
        .ok_or("no file in the local tree")?;
    want(
        file.file_ref.is_none(),
        "a foreign file has a checksum before the import",
    )?;
    want(
        !super::fixture::read_all(p, file).await?.is_empty(),
        "a local file cannot be read (import source)",
    )?;
    let o = || WriteOpts {
        on_conflict: OnConflict::Fail,
        media_type: None,
    };
    refused(
        p.write(&root.id, "x.txt", vec![1], o(), &Ctl::none()).await,
        "write to local",
        unsupported,
    )?;
    refused(
        p.create(&root.id, "x", Kind::Folder, OnConflict::Fail)
            .await,
        "create on local",
        unsupported,
    )?;
    refused(
        p.rename(&file.id, "y", OnConflict::Fail).await,
        "rename on local",
        unsupported,
    )?;
    refused(
        p.delete(&file.id, Delete::Purge).await,
        "delete on local",
        unsupported,
    )?;
    refused(
        p.copy_to(&file.id, &root.id, OnConflict::KeepBoth, &Ctl::none())
            .await,
        "copy on local",
        unsupported,
    )
}
