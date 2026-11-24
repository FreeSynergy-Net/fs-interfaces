//! The `FormSource` server of the fake (the rules of level 1: errors in the answer at `invoke`, in an exception at `page`/`blob`).
//!
//! Pattern: Fake Object (the server side of the fake: the rules of level 1 in one place).
#![allow(clippy::unused_async_trait_impl)] // the generated server trait is async

use fs_interfaces::common_capnp::problem;
use fs_interfaces::forms::{FORM_SOURCE_LEVEL, parse_key};
use fs_interfaces::formsource_capnp::{form_source, value};
use std::rc::Rc;

use crate::fake::{FakeSource, LOCKED, Mode, Refusal, refuse};
use crate::fixture::CommandKind;

type Reply = Result<(), capnp::Error>;

impl form_source::Server for FakeSource {
    async fn hello(
        self: Rc<Self>,
        _: form_source::HelloParams,
        mut r: form_source::HelloResults,
    ) -> Reply {
        let mut out = r.get();
        out.set_level(FORM_SOURCE_LEVEL);
        out.set_provider("fake");
        Ok(())
    }

    async fn page(
        self: Rc<Self>,
        p: form_source::PageParams,
        mut r: form_source::PageResults,
    ) -> Reply {
        let p = p.get()?;
        let known = parse_key(p.get_form()?.to_str()?)
            .is_some_and(|(n, v)| n == "cards-in-columns" && (v.major, v.minor) == (1, 0));
        if !known {
            return Err(refuse("unknown-form", 404, "no such form").error());
        }
        if !p.get_scope()?.to_str()?.is_empty() {
            return Err(refuse("unknown-scope", 404, "the fake has no scopes").error());
        }
        let skip = match p.get_cursor()?.get_opaque()? {
            [] => 0,
            b => std::str::from_utf8(b)
                .ok()
                .and_then(|t| t.parse::<usize>().ok())
                .ok_or_else(|| refuse("bad-arguments", 400, "bad cursor").error())?,
        };
        let limit = if p.get_limit() == 0 {
            100
        } else {
            p.get_limit() as usize
        };
        let cards = self.cards.borrow();
        let end = (skip + limit).min(cards.len());
        let mut out = r.get();
        {
            let mut page = out.reborrow().init_page();
            page.set_total(cards.len() as u64);
            page.set_has_total(true);
            page.reborrow().init_may(1).set(0, "card.create");
            let rows = cards.get(skip..end).unwrap_or_default();
            let mut list = page.init_records(
                u32::try_from(rows.len()).map_err(|_| capnp::Error::failed("too many".into()))?,
            );
            for (i, (id, title, col)) in (0u32..).zip(rows) {
                let mut view = list.reborrow().get(i);
                view.set_id(id.as_str());
                if id != LOCKED {
                    let mut may = view.reborrow().init_may(2);
                    may.set(0, "card.move");
                    may.set(1, "card.delete");
                }
                let mut values = view.init_values(2);
                for (j, (path, text)) in (0u32..).zip([("title", title), ("column", col)]) {
                    let mut v: value::Builder<'_> = values.reborrow().get(j);
                    v.set_path(path);
                    v.set_text(text.as_str());
                }
            }
        }
        if end < cards.len() {
            out.init_next().set_opaque(end.to_string().as_bytes());
        }
        Ok(())
    }

    async fn blob(
        self: Rc<Self>,
        _: form_source::BlobParams,
        _: form_source::BlobResults,
    ) -> Reply {
        Err(refuse("unsupported", 501, "the fake has no image or file field").error())
    }

    async fn invoke(
        self: Rc<Self>,
        p: form_source::InvokeParams,
        mut r: form_source::InvokeResults,
    ) -> Reply {
        let p = p.get()?;
        let (command, record) = (p.get_command()?.to_str()?, p.get_record()?.to_str()?);
        let args: Vec<(String, String)> = p
            .get_args()?
            .iter()
            .map(|v| Ok((v.get_path()?.to_string()?, v.get_text()?.to_string()?)))
            .collect::<capnp::Result<_>>()?;
        let mut out = r.get();
        match self.run(command, record, &args) {
            Ok((id, text)) => {
                let mut made = out.init_result();
                made.set_record(id.as_str());
                made.set_text(text);
                Ok(())
            }
            Err(why) if self.mode == Mode::LaxErrors => Err(why.error()),
            Err(why) => {
                out.set_failed(true);
                let mut prob: problem::Builder<'_> = out.init_problem();
                prob.set_type(why.kind.as_str());
                prob.set_title(why.kind.as_str());
                prob.set_detail(why.detail.as_str());
                prob.set_status(why.status);
                Ok(())
            }
        }
    }
}

impl FakeSource {
    /// One command: known, then allowed, then its arguments, then the record. Returns the record made or changed.
    fn run(
        &self,
        command: &str,
        record: &str,
        args: &[(String, String)],
    ) -> Result<(String, &'static str), Refusal> {
        let kind = Self::kind_of(command)
            .ok_or_else(|| refuse("unknown-command", 404, "no such command"))?;
        if kind == CommandKind::Record && record == LOCKED {
            return Err(refuse("forbidden", 403, "this card is locked"));
        }
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
        let mut cards = self.cards.borrow_mut();
        if kind == CommandKind::Set {
            let id = format!("c{}", self.next.replace_with(|n| *n + 1));
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
        cards[at].2 = arg("column", "todo");
        Ok((record.to_owned(), "moved"))
    }
}
