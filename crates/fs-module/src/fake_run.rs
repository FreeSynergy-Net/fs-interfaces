//! The commands of level 1 of the fake (`run`) and the image behind `blob`.
//!
//! Pattern: Fake Object (see `fake_server`; the rules of level 1 in one place, kept apart from the server trait).
#![allow(clippy::unused_async_trait_impl)] // the generated server trait is async

use std::rc::Rc;

use fs_interfaces::common_capnp::blob;

use crate::fake::{FakeSource, LOCKED, Refusal, SET_LOCKED, refuse};
use crate::fixture::CommandKind;

type Reply = Result<(), capnp::Error>;

impl FakeSource {
    /// Whether the rights of the fake refuse this command on this record (the ONE place of the rule; `FakeRules` asks it too):
    /// a record command on the locked card, and the set command `card.clear`.
    pub(crate) fn locked(command: &str, record: &str) -> bool {
        (Self::kind_of(command) == Some(CommandKind::Record) && record == LOCKED)
            || command == SET_LOCKED
    }

    /// The RIGHT of one command (apart from its effect): `Ok` or the `forbidden` refusal. An unknown command is no question of rights.
    pub(crate) fn rights(command: &str, record: &str) -> Result<(), Refusal> {
        if Self::locked(command, record) {
            return Err(refuse("forbidden", 403, "this card is locked"));
        }
        Ok(())
    }

    /// One command with its rights: `rights`, then `effect`. This is the level 1 server's way (the fake checks its own rights).
    pub(crate) fn run(
        &self,
        command: &str,
        record: &str,
        args: &[(String, String)],
    ) -> Result<(String, &'static str), Refusal> {
        Self::kind_of(command).ok_or_else(|| refuse("unknown-command", 404, "no such command"))?;
        Self::rights(command, record)?;
        self.effect(command, record, args)
    }

    /// The EFFECT of one command, no rights asked: known, then its arguments, then the record. Returns the record made or changed.
    /// `FakeCore` calls this one (under a wrapper the module never checks rights).
    pub(crate) fn effect(
        &self,
        command: &str,
        record: &str,
        args: &[(String, String)],
    ) -> Result<(String, &'static str), Refusal> {
        let kind = Self::kind_of(command)
            .ok_or_else(|| refuse("unknown-command", 404, "no such command"))?;
        let params = Self::commands()
            .into_iter()
            .find(|c| c.key == command)
            .map(|c| c.params)
            .unwrap_or_default();
        for (i, (name, _)) in args.iter().enumerate() {
            if !params.iter().any(|p| &p.name == name) || args[..i].iter().any(|(n, _)| n == name) {
                return Err(refuse("bad-arguments", 400, "unknown or doubled argument"));
            }
        }
        if params
            .iter()
            .any(|p| p.required && !args.iter().any(|(n, _)| n == &p.name))
        {
            return Err(refuse(
                "bad-arguments",
                400,
                "a required argument is missing",
            ));
        }
        let arg = |n: &str, d: &str| {
            args.iter()
                .find(|(k, _)| k == n)
                .map_or_else(|| d.to_owned(), |(_, v)| v.clone())
        };
        let mut cards = self.sh.cards.borrow_mut();
        if kind == CommandKind::Set {
            if command == "card.create"
                && !args.iter().any(|(n, _)| n == "lane")
                && self.boards != 1
            {
                return Err(refuse(
                    "bad-arguments",
                    400,
                    "lane is missing and there is not exactly one board",
                ));
            }
            let id = format!("c{}", self.sh.next.replace_with(|n| *n + 1));
            cards.push((id.clone(), arg("title", ""), arg("column", "todo")));
            return Ok((id, "created"));
        }
        let at = cards
            .iter()
            .position(|c| c.0 == record)
            .ok_or_else(|| refuse("unknown-record", 404, "no such record"))?;
        if command == "card.delete" {
            cards.remove(at);
            return Ok((String::new(), "deleted"));
        }
        // Resolve the neighbours BEFORE anything changes: a refusal leaves the set as it was.
        let neighbour = |n: &str| {
            let id = arg(n, "");
            if id.is_empty() {
                return Ok(None);
            }
            cards
                .iter()
                .position(|c| c.0 == id)
                .map(Some)
                .ok_or_else(|| refuse("unknown-record", 404, "no such neighbour record"))
        };
        let (after, before) = (neighbour("after")?, neighbour("before")?);
        let mut card = cards.remove(at);
        card.2 = arg("column", "todo");
        // The place is named by the neighbours (record ids), never by an integer index. Indexes are of the set before the removal.
        let to = match (after, before) {
            (Some(a), _) => a + 1 - usize::from(a > at),
            (None, Some(b)) => b - usize::from(b > at),
            (None, None) => at,
        };
        cards.insert(to, card);
        Ok((record.to_owned(), "moved"))
    }
}

/// The image of card `c1`: bytes behind a `Blob` capability.
pub(crate) struct Image;

impl blob::Server for Image {
    async fn size(self: Rc<Self>, _: blob::SizeParams, mut r: blob::SizeResults) -> Reply {
        r.get().set_bytes(crate::fake::IMAGE.len() as u64);
        r.get().set_media_type("image/png");
        Ok(())
    }

    async fn read(self: Rc<Self>, p: blob::ReadParams, mut r: blob::ReadResults) -> Reply {
        let p = p.get()?;
        let at = usize::try_from(p.get_offset()).unwrap_or(usize::MAX);
        let end = at
            .saturating_add(p.get_length() as usize)
            .min(crate::fake::IMAGE.len());
        // At most 4 bytes per answer ("may answer fewer bytes than asked") so a reader must loop.
        let end = end.min(at.saturating_add(4));
        r.get()
            .set_data(crate::fake::IMAGE.get(at..end).unwrap_or_default());
        Ok(())
    }
}
