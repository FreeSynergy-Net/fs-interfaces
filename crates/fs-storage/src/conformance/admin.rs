//! Manager, operator, bot and foreign-tree rules (security). A manager (admin level (a)/(b), no server access) sees the
//! structure of group folders and only `usage` of private ones; the operator (server access) reads everything; a bot has
//! nothing; `local` is no place to save to.
//!
//! Pattern: Rule set. User decisions of 23.11.2026 (files addendum, decision 16): whoever can read it technically as root
//! may read it here, and the interface says so openly; everyone else stays really limited because they cannot read the
//! database. For a manager the content of a group needs an intervention with a reason (ADR 0095 point 7, not part of this
//! contract: `open` without it is Forbidden). A log entry for the operator's reads is information only: it records access
//! through the program and protects nothing (whoever is root can read past it).

use super::fixture::{
    Check, Fixture, all, forbidden, read_all, refused, root_of, s, sha, walk, want,
};
use crate::ctl::{Ctl, MemSource};
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
        .filter(|(p, e)| p.starts_with("Helfa") && e.kind == Kind::File)
    {
        s(p.stat(&e.id).await)?;
        refused(
            p.open(&e.id, &Ctl::none()).await,
            &format!("admin opens {path} without an intervention"),
            forbidden,
        )?;
    }
    super::changes::none_allowed(p, &*fx.anna, "manager").await
}

pub(super) async fn operator(fx: &Fixture) -> Check {
    let p = &*fx.operator;
    let roots = s(p.roots().await)?;
    want(
        roots.iter().filter(|r| r.kind == RootKind::Mine).count() >= 5,
        "the operator does not get every \"My files\"",
    )?;
    want(
        roots.iter().any(|r| r.kind == RootKind::Group),
        "the operator does not get the group folder",
    )?;
    let before = walk(&*fx.anna).await?;
    let mut files = 0;
    for (path, e) in before.iter().filter(|(p, _)| !p.starts_with("Papierkorb")) {
        s(p.stat(&e.id).await).map_err(|w| format!("operator stat {path}: {w}"))?;
        if e.kind == Kind::Folder {
            s(p.list(&e.id, PageReq::first(5)).await)
                .map_err(|w| format!("operator list {path}: {w}"))?;
        } else if e.kind == Kind::File {
            let got = read_all(p, e)
                .await
                .map_err(|w| format!("operator open {path}: {w}"))?;
            let f = e.file_ref.as_ref().ok_or("no file_ref")?;
            want(
                sha(&got) == f.sha256,
                &format!("{path}: sha256 differs for the operator"),
            )?;
            files += 1;
        }
    }
    want(
        files == 14,
        &format!("the operator read {files} files of Anna's tree, expected 14 (without the trash)"),
    )?;
    want(
        walk(p)
            .await?
            .iter()
            .any(|(path, _)| path == "Papierkorb/Alt.txt"),
        "the operator cannot read another's trash",
    )?;
    want(
        walk(&*fx.anna).await? == before,
        "the operator changed the tree",
    )?;
    super::changes::none_allowed(p, &*fx.anna, "operator").await
}

pub(super) async fn bot(fx: &Fixture) -> Check {
    let p = &*fx.bot;
    refused(
        p.stat(&crate::ids::EntryId("trash".into())).await,
        "a bot stats the trash (it would own it)",
        forbidden,
    )?;
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
    for (path, e) in all_entries
        .iter()
        .filter(|(p, _)| p.starts_with("Meine Dateien/"))
    {
        refused(
            p.stat(&e.id).await,
            &format!("bot (also in the operator list) stats {path}"),
            forbidden,
        )?;
    }
    let group = root_of(&*fx.anna, RootKind::Group).await?;
    let o = WriteOpts {
        generator: None,
        on_conflict: OnConflict::Fail,
        media_type: None,
    };
    refused(
        p.write(
            &group,
            "bot.txt",
            &mut MemSource::from(vec![1]),
            o,
            &Ctl::none(),
        )
        .await,
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
        generator: None,
        on_conflict: OnConflict::Fail,
        media_type: None,
    };
    refused(
        p.write(
            &root.id,
            "x.txt",
            &mut MemSource::from(vec![1]),
            o(),
            &Ctl::none(),
        )
        .await,
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
