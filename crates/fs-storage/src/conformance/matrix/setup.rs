//! The set-up of one matrix cell: every person gets an own folder, files, trashed entries; Anna also fills the group.
//!
//! Pattern: Builder (all through the contract itself: `write`, `create`, `delete`), so a provider is prepared the same way.

use super::model::{Actor, Target};
use crate::conformance::fixture::{Fixture, root_of, s};
use crate::ctl::{Ctl, MemSource};
use crate::entry::{Delete, Kind, OnConflict, PageReq, RootKind, WriteOpts};
use crate::ids::EntryId;
use crate::provider::StorageProvider;

/// What one person owns in the cell.
#[derive(Clone, Debug)]
pub(super) struct Own {
    pub root: EntryId,
    pub trash: EntryId,
    pub folder: EntryId,
    pub sub: EntryId,
    pub file: EntryId,
    pub file2: EntryId,
    pub tfile: EntryId,
    pub tfolder: EntryId,
    pub tsub: EntryId,
}

/// The ids of one cell.
pub(super) struct Setup {
    pub own: [Option<Own>; 5],
    pub group_root: EntryId,
    pub gfolder: EntryId,
    pub gfile: EntryId,
    pub trashed_gfile: EntryId,
    pub trashed_gfolder: EntryId,
    pub link: EntryId,
    pub local_root: EntryId,
    pub local_file: EntryId,
}

/// The provider of an actor.
pub(super) fn provider(fx: &Fixture, a: Actor) -> &dyn StorageProvider {
    match a {
        Actor::E => &*fx.anna,
        Actor::M => &*fx.ben,
        Actor::F => &*fx.fremd,
        Actor::V => &*fx.admin,
        Actor::B => &*fx.operator,
        Actor::K => &*fx.bot,
    }
}

fn opts() -> WriteOpts {
    WriteOpts {
        generator: None,
        on_conflict: OnConflict::Fail,
        media_type: None,
    }
}

async fn file(p: &dyn StorageProvider, at: &EntryId, name: &str) -> Result<EntryId, String> {
    let e = p
        .write(
            at,
            name,
            &mut MemSource::from(vec![1, 2, 3]),
            opts(),
            &Ctl::none(),
        )
        .await;
    Ok(s(e)?.id)
}

async fn folder(p: &dyn StorageProvider, at: &EntryId, name: &str) -> Result<EntryId, String> {
    Ok(s(p.create(at, name, Kind::Folder, OnConflict::Fail).await)?.id)
}

async fn trash(p: &dyn StorageProvider, id: &EntryId) -> Result<(), String> {
    s(p.delete(id, Delete::Trash).await)
}

async fn own(p: &dyn StorageProvider) -> Result<Own, String> {
    let roots = s(p.roots().await)?;
    let tr = roots
        .iter()
        .find(|r| r.kind == RootKind::Trash)
        .ok_or("no trash root")?;
    let mine = roots
        .iter()
        .find(|r| r.kind == RootKind::Mine && r.owner.id == tr.owner.id)
        .ok_or("no own root")?;
    let (root, trash_id) = (mine.id.clone(), tr.id.clone());
    let f = folder(p, &root, "OF").await?;
    let sub = folder(p, &f, "sub").await?;
    let file_id = file(p, &root, "of.txt").await?;
    let file2 = file(p, &sub, "mv.txt").await?;
    let tfile = file(p, &root, "ot.txt").await?;
    trash(p, &tfile).await?;
    let tfolder = folder(p, &root, "otf").await?;
    let tsub = folder(p, &tfolder, "otfs").await?;
    trash(p, &tfolder).await?;
    Ok(Own {
        root,
        trash: trash_id,
        folder: f,
        sub,
        file: file_id,
        file2,
        tfile,
        tfolder,
        tsub,
    })
}

/// Build the set-up of one cell on a fresh fixture.
pub(super) async fn build(fx: &Fixture) -> Result<Setup, String> {
    let mut owns: [Option<Own>; 5] = [None, None, None, None, None];
    for (i, a) in [Actor::E, Actor::M, Actor::F, Actor::V, Actor::B]
        .into_iter()
        .enumerate()
    {
        owns[i] = Some(own(provider(fx, a)).await?);
    }
    let p = &*fx.anna;
    let group_root = root_of(p, RootKind::Group).await?;
    let gfolder = folder(p, &group_root, "GF").await?;
    let gfile = file(p, &gfolder, "gf.txt").await?;
    let trashed_gfile = file(p, &gfolder, "gt.txt").await?;
    trash(p, &trashed_gfile).await?;
    let trashed_gfolder = folder(p, &group_root, "GTF").await?;
    trash(p, &trashed_gfolder).await?;
    let mine = owns[0]
        .as_ref()
        .map(|o| o.root.clone())
        .ok_or("no own root")?;
    let link = s(p.list(&mine, PageReq::first(100)).await)?
        .items
        .into_iter()
        .find(|e| e.kind == Kind::Link)
        .map(|e| e.id)
        .ok_or("the sample has no link")?;
    let lroots = s(fx.local.roots().await)?;
    let local_root = lroots
        .first()
        .map(|r| r.id.clone())
        .ok_or("no local root")?;
    let items = s(fx.local.list(&local_root, PageReq::first(10)).await)?.items;
    let pick = |k: Kind| {
        items
            .iter()
            .find(|e| e.kind == k)
            .map(|e| e.id.clone())
            .ok_or("local tree incomplete")
    };
    Ok(Setup {
        own: owns,
        group_root,
        gfolder,
        gfile,
        trashed_gfile,
        trashed_gfolder,
        link,
        local_root,
        local_file: pick(Kind::File)?,
    })
}

fn idx(a: Actor) -> Option<usize> {
    match a {
        Actor::E => Some(0),
        Actor::M => Some(1),
        Actor::F => Some(2),
        Actor::V => Some(3),
        Actor::B => Some(4),
        Actor::K => None,
    }
}

impl Setup {
    /// The own set-up of an actor (none for the bot).
    pub(super) fn of(&self, a: Actor) -> Option<&Own> {
        idx(a).and_then(|i| self.own[i].as_ref())
    }

    /// The other person the foreign targets belong to: Ben for the owner, Anna for everyone else.
    fn other(&self, a: Actor) -> Option<&Own> {
        self.own[usize::from(a == Actor::E)].as_ref()
    }

    /// The id of target `t` for actor `a`; `folder` asks for the folder variant. `None`: the target does not exist for this
    /// actor ("n/a" in the table).
    pub(super) fn target(&self, a: Actor, t: Target, folder: bool) -> Option<EntryId> {
        let pick = |o: Option<&Own>, f: fn(&Own) -> &EntryId, g: fn(&Own) -> &EntryId| {
            o.map(|o| if folder { f(o) } else { g(o) }.clone())
        };
        match t {
            Target::T1 => pick(self.of(a), |o| &o.folder, |o| &o.file),
            Target::T2 => pick(self.other(a), |o| &o.folder, |o| &o.file),
            Target::T3 => self.of(a).map(|o| o.root.clone()),
            Target::T4 => self.other(a).map(|o| o.root.clone()),
            Target::T5 => Some(self.group_root.clone()),
            Target::T6 => Some(if folder {
                self.gfolder.clone()
            } else {
                self.gfile.clone()
            }),
            Target::T7 => Some(
                self.of(a)
                    .map_or_else(|| EntryId("trash".into()), |o| o.trash.clone()),
            ),
            Target::T8 => pick(self.of(a), |o| &o.tfolder, |o| &o.tfile),
            Target::T8B => self.of(a).map(|o| o.tsub.clone()),
            Target::T9 => pick(self.other(a), |o| &o.tfolder, |o| &o.tfile),
            Target::T10 => Some(if folder {
                self.trashed_gfolder.clone()
            } else {
                self.trashed_gfile.clone()
            }),
            Target::T11 => (a == Actor::E).then(|| self.link.clone()),
            Target::T12 => (a != Actor::E).then(|| self.link.clone()),
            Target::T13 => Some(if folder {
                self.local_root.clone()
            } else {
                self.local_file.clone()
            }),
        }
    }
}
