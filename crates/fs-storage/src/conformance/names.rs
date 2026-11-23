//! Name and path rules (security): no traversal, no hidden control characters, a link is never followed.
//!
//! Pattern: Rule set. Every refused call must also leave the tree as it was.

use super::fixture::{
    Check, Fixture, all, at, invalid_name, link, refused, root_of, s, walk, want,
};
use crate::ctl::{Ctl, MemSource};
use crate::entry::{Delete, Kind, OnConflict, PageReq, RootKind, WriteOpts};

fn bad_list() -> Vec<String> {
    let mut v: Vec<String> = [
        "",
        ".",
        "..",
        "a/b",
        "a\\b",
        "../x",
        "x\0y",
        "x\u{7}",
        "x\u{85}",
        "evil\u{202E}fdp.exe",
        ".fs-files",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    v.push("n".repeat(256));
    v
}

pub(super) async fn bad_names(fx: &Fixture) -> Check {
    let dir = at(&*fx.anna, RootKind::Mine, "Dokumente").await?;
    let file = at(&*fx.anna, RootKind::Mine, "Dokumente/Notizen.md").await?;
    let before = walk(&*fx.anna).await?;
    let o = || WriteOpts {
        generator: None,
        on_conflict: OnConflict::Fail,
        media_type: None,
    };
    for name in bad_list() {
        let w = format!("name {name:?}");
        refused(
            fx.anna
                .create(&dir.id, &name, Kind::Folder, OnConflict::Fail)
                .await,
            &w,
            invalid_name,
        )?;
        refused(
            fx.anna
                .write(
                    &dir.id,
                    &name,
                    &mut MemSource::from(vec![1]),
                    o(),
                    &Ctl::none(),
                )
                .await,
            &w,
            invalid_name,
        )?;
        refused(
            fx.anna.rename(&file.id, &name, OnConflict::Fail).await,
            &w,
            invalid_name,
        )?;
    }
    want(
        walk(&*fx.anna).await? == before,
        "a refused name changed the tree",
    )
}

pub(super) async fn traversal(fx: &Fixture) -> Check {
    let root = root_of(&*fx.anna, RootKind::Mine).await?;
    let paths = [
        "..",
        "Dokumente/..",
        "Dokumente/../Bilder",
        "Dokumente/../../Helfa",
        "/Dokumente",
        "Dokumente//Notizen.md",
        "Dokumente\\..\\Bilder",
        "Dokumente/\0",
        "./Dokumente",
    ];
    for p in paths {
        refused(
            fx.anna.resolve_path(&root, p).await,
            &format!("path {p:?}"),
            invalid_name,
        )?;
    }
    s(fx.anna.resolve_path(&root, "Dokumente/Notizen.md").await).map(|_| ())
}

pub(super) async fn links(fx: &Fixture) -> Check {
    let root = root_of(&*fx.anna, RootKind::Mine).await?;
    let ln = all(&*fx.anna, &root)
        .await?
        .into_iter()
        .find(|e| e.kind == Kind::Link)
        .ok_or("the sample has no link")?;
    let group = at(&*fx.anna, RootKind::Group, "").await?;
    let before = all(&*fx.anna, &group.id).await?;
    refused(
        fx.anna.list(&ln.id, PageReq::first(10)).await,
        "list a link",
        link,
    )?;
    refused(
        fx.anna.open(&ln.id, &Ctl::none()).await,
        "open a link",
        link,
    )?;
    refused(
        fx.anna
            .resolve_path(&root, &format!("{}/Regeln/Konfliktstufen.md", ln.name))
            .await,
        "walk through a link",
        link,
    )?;
    let o = WriteOpts {
        generator: None,
        on_conflict: OnConflict::Fail,
        media_type: None,
    };
    refused(
        fx.anna
            .write(
                &ln.id,
                "x.txt",
                &mut MemSource::from(vec![1]),
                o,
                &Ctl::none(),
            )
            .await,
        "write through a link",
        link,
    )?;
    let dok = at(&*fx.anna, RootKind::Mine, "Dokumente").await?;
    let copy = s(fx
        .anna
        .copy_to(&ln.id, &dok.id, OnConflict::Fail, &Ctl::none())
        .await)?;
    want(copy.kind == Kind::Link, "a copied link must stay a link")?;
    s(fx.anna.delete(&ln.id, Delete::Purge).await)?;
    s(fx.anna.delete(&copy.id, Delete::Purge).await)?;
    want(
        all(&*fx.anna, &group.id).await? == before,
        "copy or delete of a link touched the target",
    )
}
