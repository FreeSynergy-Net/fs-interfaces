//! The big-file rule: a file of `MAX_READ + 1` bytes must come out in pieces of at most `MAX_READ`.
//!
//! Pattern: Rule helper.

use super::fixture::{Check, Fixture, read_range, s, want};
use crate::ctl::Ctl;
use crate::entry::RootKind;

/// A file of `MAX_READ + 1` bytes: no read hands it out whole.
pub(super) async fn big_file(fx: &Fixture) -> Check {
    let p = &*fx.anna;
    let dir = super::fixture::at(p, RootKind::Mine, "Musik").await?;
    let body: Vec<u8> = (0..=crate::ctl::MAX_READ)
        .map(|i| (i % 251) as u8)
        .collect();
    let o = crate::entry::WriteOpts {
        generator: None,
        on_conflict: crate::entry::OnConflict::Fail,
        media_type: None,
    };
    let e = s(p
        .write(
            &dir.id,
            "gross.bin",
            &mut crate::ctl::MemSource::from(body.clone()),
            o,
            &Ctl::none(),
        )
        .await)?;
    let blob = s(p.open(&e.id, &Ctl::none()).await)?;
    let first = s(blob.read(0, u64::MAX).await)?;
    want(
        !first.is_empty() && first.len() as u64 <= crate::ctl::MAX_READ,
        "one read of a file bigger than MAX_READ must stay within MAX_READ",
    )?;
    want(
        read_range(&*blob, body.len() as u64, 0, u64::MAX).await? == body,
        "a big file differs after short reads",
    )
}
