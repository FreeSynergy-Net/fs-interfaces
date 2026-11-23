//! Read rules: roots, paging, the 12 sample files, sha256 against the bytes, owners, stat, honest capabilities.
//!
//! Pattern: Rule set (one function per rule, `Ok` or the reason).

use super::fixture::{
    Check, Fixture, all, read_all, read_range, refused, root_of, s, sha, walk, want,
};
use crate::ctl::Ctl;
use crate::entry::{Kind, PageReq, RootKind};
use crate::error::Error;
use fs_types::PersonArt;
use std::sync::Arc;

pub(super) async fn roots_shape(fx: &Fixture) -> Check {
    let kinds = |v: Vec<crate::Root>| v.iter().map(|r| r.kind).collect::<Vec<_>>();
    let a = s(fx.anna.roots().await)?;
    for k in [RootKind::Mine, RootKind::Group, RootKind::Trash] {
        want(
            a.iter().any(|r| r.kind == k),
            &format!("anna has no root {k:?}"),
        )?;
    }
    want(
        a.iter()
            .any(|r| r.kind == RootKind::Mine && r.owner.art == PersonArt::Person),
        "Mine is not owned by a person",
    )?;
    want(
        a.iter()
            .any(|r| r.kind == RootKind::Group && r.owner.art == PersonArt::Group),
        "group folder is not owned by a group",
    )?;
    want(
        kinds(s(fx.ben.roots().await)?).contains(&RootKind::Group),
        "a member does not see the group folder",
    )?;
    want(
        !kinds(s(fx.fremd.roots().await)?).contains(&RootKind::Group),
        "a stranger sees the group folder",
    )
}

pub(super) async fn paging(fx: &Fixture) -> Check {
    let root = root_of(&*fx.anna, RootKind::Mine).await?;
    let big = s(fx.anna.list(&root, PageReq::first(100)).await)?;
    want(
        big.next.is_none() && !big.items.is_empty(),
        "one big page must be complete",
    )?;
    let paged = all(&*fx.anna, &root).await?;
    want(
        paged == big.items,
        "pages of size 2 differ from one big page",
    )?;
    let small = s(fx.anna.list(&root, PageReq::first(1)).await)?;
    want(
        small.items.len() == 1 && small.next.is_some(),
        "a page of size 1 must carry a cursor",
    )?;
    refused(
        fx.anna.list(&root, PageReq::first(0)).await,
        "page size 0",
        |e| matches!(e, Error::Invalid(_)),
    )
}

pub(super) async fn tree_files(fx: &Fixture) -> Check {
    let files: Vec<_> = walk(&*fx.anna)
        .await?
        .into_iter()
        .filter(|(_, e)| e.kind == Kind::File)
        .collect();
    want(
        files.len() == 15,
        &format!("sample has 15 files (12 + 3 images), found {}", files.len()),
    )?;
    let mut types: Vec<_> = files
        .iter()
        .filter_map(|(_, e)| e.media_type.as_ref().map(|m| m.0.clone()))
        .collect();
    types.sort();
    types.dedup();
    want(types.len() >= 8, "the sample files must be of mixed types")
}

pub(super) async fn file_sha256(fx: &Fixture) -> Check {
    for (path, e) in walk(&*fx.anna)
        .await?
        .into_iter()
        .filter(|(_, e)| e.kind == Kind::File)
    {
        let f = e.file_ref.as_ref().ok_or(format!("{path}: no file_ref"))?;
        let bytes = read_all(&*fx.anna, &e).await?;
        want(
            sha(&bytes) == f.sha256,
            &format!("{path}: sha256 does not match the bytes"),
        )?;
        want(
            e.size == Some(bytes.len() as u64),
            &format!("{path}: size does not match"),
        )?;
        let blob = s(fx.anna.open(&e.id, &Ctl::none()).await)?;
        want(
            read_range(&*blob, bytes.len() as u64, 2, 5).await? == bytes[2..7],
            &format!("{path}: range read is wrong"),
        )?;
        want(
            s(blob.read(bytes.len() as u64 + 9, 5).await)?.is_empty(),
            &format!("{path}: read behind the end must be empty"),
        )?;
    }
    Ok(())
}

pub(super) async fn owner_kinds(fx: &Fixture) -> Check {
    for (path, e) in walk(&*fx.anna).await? {
        let group = path.starts_with("Helfa");
        let right = if group {
            e.owner.art == PersonArt::Group
        } else {
            e.owner.art == PersonArt::Person
        };
        want(right, &format!("{path}: wrong owner kind"))?;
    }
    Ok(())
}

pub(super) async fn stat_matches(fx: &Fixture) -> Check {
    for (path, e) in walk(&*fx.anna).await? {
        want(
            s(fx.anna.stat(&e.id).await)? == e,
            &format!("{path}: stat differs from list"),
        )?;
    }
    Ok(())
}

pub(super) async fn caps_honest(fx: &Fixture) -> Check {
    let root = root_of(&*fx.anna, RootKind::Mine).await?;
    let c = fx.anna.capabilities();
    if !c.watch {
        refused(
            fx.anna.watch(&root, Arc::new(|_| {})).await,
            "watch without the flag",
            |e| matches!(e, Error::Unsupported { .. }),
        )?;
    }
    if !c.delta_since {
        refused(
            fx.anna.changes(None).await,
            "changes without the flag",
            |e| matches!(e, Error::Unsupported { .. }),
        )?;
    }
    Ok(())
}

pub(super) async fn blob_range(fx: &Fixture) -> Check {
    let e = super::fixture::at(&*fx.anna, RootKind::Mine, "Dokumente/Notizen.md").await?;
    let bytes = read_all(&*fx.anna, &e).await?;
    let len = bytes.len() as u64;
    let blob = s(fx.anna.open(&e.id, &Ctl::none()).await)?;
    let first = s(blob.read(0, u64::MAX).await)?;
    want(
        !first.is_empty()
            && first.len() as u64 <= len.min(crate::ctl::MAX_READ)
            && first == bytes[..first.len()],
        "read(0, u64::MAX) must answer a non-empty prefix of at most MAX_READ bytes",
    )?;
    for (offset, length) in [
        (0, 1),
        (3, 4),
        (len - 1, 1),
        (len - 2, 50),
        (5, 0),
        (0, u64::MAX),
        (3, u64::MAX),
    ] {
        let from = usize::try_from(offset.min(len)).unwrap_or(0);
        let to = usize::try_from(offset.saturating_add(length).min(len)).unwrap_or(0);
        let got = read_range(&*blob, len, offset, length).await?;
        want(
            got == bytes[from..to],
            &format!("range ({offset}, {length}) is wrong"),
        )?;
    }
    for offset in [len, len + 1, u64::MAX] {
        want(
            s(blob.read(offset, 10).await)?.is_empty(),
            &format!("read at {offset} (end or behind it) must be empty"),
        )?;
    }
    Ok(())
}

pub(super) async fn scope_generator(fx: &Fixture) -> Check {
    want(
        fx.anna.capabilities().scope,
        "the provider does not offer scope",
    )?;
    let hits = s(fx.anna.scope("capture", PageReq::first(100)).await)?;
    want(
        hits.items.len() == 3 && hits.next.is_none(),
        &format!("anna should see 3 captures, got {}", hits.items.len()),
    )?;
    want(
        hits.items
            .iter()
            .all(|e| e.generator.as_deref() == Some("capture")),
        "scope returned another generator",
    )?;
    let places: std::collections::HashSet<_> = walk(&*fx.anna)
        .await?
        .into_iter()
        .filter(|(_, e)| e.generator.is_some())
        .map(|(p, _)| p.rsplit_once('/').map(|x| x.0.to_owned()))
        .collect();
    want(
        places.len() == 3,
        "the gallery is a scope: the captures lie in 3 different folders",
    )?;
    let mut ids = Vec::new();
    let mut req = PageReq::first(1);
    loop {
        let page = s(fx.anna.scope("capture", req.clone()).await)?;
        ids.extend(page.items.into_iter().map(|e| e.id));
        match page.next {
            Some(c) => req.cursor = Some(c),
            None => break,
        }
    }
    want(
        ids == hits.items.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
        "paged scope differs from one page",
    )?;
    want(
        s(fx.anna.scope("nothing", PageReq::first(5)).await)?
            .items
            .is_empty(),
        "an unknown generator must give nothing",
    )?;
    want(
        s(fx.ben.scope("capture", PageReq::first(5)).await)?
            .items
            .len()
            == 1,
        "a member sees only the group's capture",
    )?;
    want(
        s(fx.fremd.scope("capture", PageReq::first(5)).await)?
            .items
            .is_empty(),
        "a stranger sees captures through scope",
    )?;
    want(
        s(fx.admin.scope("capture", PageReq::first(5)).await)?
            .items
            .is_empty(),
        "a manager sees private content through scope",
    )?;
    want(
        s(fx.operator.scope("capture", PageReq::first(5)).await)?
            .items
            .len()
            == 3,
        "the operator must see all 3 captures",
    )
}
