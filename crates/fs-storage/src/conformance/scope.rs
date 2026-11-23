//! The scope rule: a gallery is everything of one generator the asker may read, wherever it lies.
//!
//! Pattern: Rule (one function, `Ok` or the reason).

use super::fixture::{Check, Fixture, s, walk, want};
use crate::entry::{PageReq, RootKind};

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
    let capture = super::fixture::at(&*fx.anna, RootKind::Mine, "Aufnahme-1.png").await?;
    s(fx.anna
        .delete(&capture.id, crate::entry::Delete::Trash)
        .await)?;
    want(
        s(fx.anna.scope("capture", PageReq::first(5)).await)?
            .items
            .len()
            == 2,
        "a trashed capture must leave the scope",
    )?;
    s(fx.anna
        .restore(&capture.id, None, crate::entry::OnConflict::Fail)
        .await)?;
    want(
        s(fx.operator.scope("capture", PageReq::first(5)).await)?
            .items
            .len()
            == 3,
        "the operator must see all 3 captures",
    )
}
