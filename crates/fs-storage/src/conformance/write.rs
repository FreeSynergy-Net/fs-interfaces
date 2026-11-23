//! Write rules: never replace, round trip with sha256, trash and purge, move and copy, delta, cancel.
//!
//! Pattern: Rule set.

use super::fixture::{Check, Fixture, all, at, conflict, read_all, refused, s, sha, walk, want};
use crate::ctl::Ctl;
use crate::entry::{ChangeKind, Delete, Kind, OnConflict, RootKind, Token, WriteOpts};
use crate::error::Error;
use fs_types::MediaType;

fn opts(on: OnConflict) -> WriteOpts {
    WriteOpts {
        on_conflict: on,
        media_type: Some(MediaType("text/plain".into())),
    }
}

pub(super) async fn never_replace(fx: &Fixture) -> Check {
    let p = &*fx.anna;
    let dir = at(p, RootKind::Mine, "Dokumente").await?;
    let old = at(p, RootKind::Mine, "Dokumente/Notizen.md").await?;
    let old_bytes = read_all(p, &old).await?;
    refused(
        p.write(
            &dir.id,
            "Notizen.md",
            vec![9],
            opts(OnConflict::Fail),
            &Ctl::none(),
        )
        .await,
        "same name",
        conflict,
    )?;
    want(
        read_all(p, &at(p, RootKind::Mine, "Dokumente/Notizen.md").await?).await? == old_bytes,
        "the old file was replaced",
    )?;
    let both = s(p
        .write(
            &dir.id,
            "Notizen.md",
            vec![9],
            opts(OnConflict::KeepBoth),
            &Ctl::none(),
        )
        .await)?;
    want(
        both.name == "Notizen (2).md",
        &format!("keepBoth gave {:?}", both.name),
    )?;
    refused(
        p.create(&dir.id, "Dokumente", Kind::Folder, OnConflict::Fail)
            .await
            .and(
                p.create(&dir.id, "Notizen.md", Kind::Folder, OnConflict::Fail)
                    .await,
            ),
        "folder on a file name",
        conflict,
    )
}

pub(super) async fn roundtrip(fx: &Fixture) -> Check {
    let p = &*fx.anna;
    let dir = at(p, RootKind::Mine, "Bilder").await?;
    let e = s(p
        .write(
            &dir.id,
            "neu.txt",
            b"hello".to_vec(),
            opts(OnConflict::Fail),
            &Ctl::none(),
        )
        .await)?;
    let f = e.file_ref.clone().ok_or("no file_ref after write")?;
    want(
        f.sha256 == sha(b"hello") && e.size == Some(5),
        "file_ref or size wrong after write",
    )?;
    want(e.owner == dir.owner, "the owner must be the folder's owner")?;
    want(
        read_all(p, &e).await? == b"hello",
        "bytes differ after write",
    )
}

pub(super) async fn trash_restore(fx: &Fixture) -> Check {
    let p = &*fx.anna;
    let f = at(p, RootKind::Mine, "Dokumente/Lebenslauf.pdf").await?;
    let dir = at(p, RootKind::Mine, "Dokumente").await?;
    s(p.delete(&f.id, Delete::Trash).await)?;
    want(
        !all(p, &dir.id).await?.iter().any(|e| e.id == f.id),
        "a trashed file is still listed",
    )?;
    want(
        walk(p)
            .await?
            .iter()
            .any(|(path, e)| path.starts_with("Papierkorb/") && e.id == f.id),
        "a trashed file is not in the trash",
    )?;
    let back = s(p.restore(&f.id, None, OnConflict::Fail).await)?;
    want(
        back.id == f.id && back.name == f.name,
        "restore gives another entry",
    )?;
    s(p.delete(&f.id, Delete::Purge).await)?;
    refused(
        p.restore(&f.id, None, OnConflict::Fail).await,
        "restore after purge",
        |e| matches!(e, Error::NotFound(_)),
    )?;
    let pics = at(p, RootKind::Mine, "Bilder").await?;
    let kid = at(p, RootKind::Mine, "Bilder/Urlaub.jpg").await?;
    s(p.delete(&pics.id, Delete::Purge).await)?;
    refused(p.stat(&kid.id).await, "child of a purged folder", |e| {
        matches!(e, Error::NotFound(_))
    })
}

pub(super) async fn move_copy(fx: &Fixture) -> Check {
    let p = &*fx.anna;
    let dok = at(p, RootKind::Mine, "Dokumente").await?;
    let bild = at(p, RootKind::Mine, "Bilder").await?;
    refused(
        p.move_to(&dok.id, &dok.id, OnConflict::Fail).await,
        "move into itself",
        |e| matches!(e, Error::Cycle(_)),
    )?;
    let sub = s(p
        .create(&dok.id, "Unter", Kind::Folder, OnConflict::Fail)
        .await)?;
    refused(
        p.move_to(&dok.id, &sub.id, OnConflict::Fail).await,
        "move into a child",
        |e| matches!(e, Error::Cycle(_)),
    )?;
    let c = s(p
        .copy_to(&dok.id, &bild.id, OnConflict::Fail, &Ctl::none())
        .await)?;
    let src = all(p, &dok.id).await?;
    let dst = all(p, &c.id).await?;
    want(
        src.len() == dst.len(),
        "a copied folder must carry its content",
    )?;
    for (a, b) in src.iter().zip(&dst) {
        want(
            a.name == b.name && a.id != b.id,
            "copy keeps names and makes new ids",
        )?;
        want(
            a.file_ref.as_ref().map(|f| f.sha256) == b.file_ref.as_ref().map(|f| f.sha256),
            "copy keeps the sha256",
        )?;
    }
    let moved = s(p.move_to(&c.id, &dok.id, OnConflict::Fail).await)?;
    want(
        all(p, &dok.id).await?.iter().any(|e| e.id == moved.id),
        "a moved entry is not below its new folder",
    )?;
    let r = s(p.rename(&moved.id, "Neu", OnConflict::Fail).await)?;
    want(r.name == "Neu" && r.id == moved.id, "rename keeps the id")
}

pub(super) async fn delta(fx: &Fixture) -> Check {
    let p = &*fx.anna;
    let t0 = s(p.changes(None).await)?.token;
    let dir = at(p, RootKind::Mine, "Musik").await?;
    let e = s(p
        .write(
            &dir.id,
            "x.txt",
            vec![1],
            opts(OnConflict::Fail),
            &Ctl::none(),
        )
        .await)?;
    let set = s(p.changes(Some(&t0)).await)?;
    want(
        set.changes
            .iter()
            .any(|c| c.id == e.id && c.kind == ChangeKind::Created && c.kind.as2() == "Create"),
        "the new entry is not in the delta",
    )?;
    want(
        s(p.changes(Some(&set.token)).await)?.changes.is_empty(),
        "the delta repeats old changes",
    )?;
    want(
        fx.fremd
            .changes(Some(&t0))
            .await
            .is_ok_and(|c| c.changes.is_empty()),
        "a stranger sees changes of others",
    )?;
    refused(
        p.changes(Some(&Token("999999".into()))).await,
        "stale token",
        |e| matches!(e, Error::Changed),
    )
}

pub(super) async fn cancel(fx: &Fixture) -> Check {
    let p = &*fx.anna;
    let dir = at(p, RootKind::Mine, "Musik").await?;
    let before = all(p, &dir.id).await?;
    let ctl = Ctl::none();
    ctl.cancel();
    refused(
        p.write(&dir.id, "x.txt", vec![1], opts(OnConflict::Fail), &ctl)
            .await,
        "cancelled write",
        |e| matches!(e, Error::Cancelled),
    )?;
    want(
        all(p, &dir.id).await? == before,
        "a cancelled write left a file",
    )
}
