//! The `FormSource` server of the fake (the rules of level 1: errors in the answer at `invoke`, in an exception at `page`/`blob`).
//!
//! Pattern: Fake Object (the server side of the fake: the rules of level 1 in one place).
#![allow(clippy::unused_async_trait_impl)] // the generated server trait is async

use fs_interfaces::common_capnp::problem;
use fs_interfaces::forms::parse_key;
use fs_interfaces::formsource_capnp::{form_source, value};
use std::rc::Rc;

use crate::fake::{FakeSource, LOCKED, Mode, refuse};
use crate::fake_l2::{self, Outcome};
use crate::fake_l2_wire::{self, Sub};
use crate::fake_run::Image;

type Reply = Result<(), capnp::Error>;

impl form_source::Server for FakeSource {
    async fn hello(
        self: Rc<Self>,
        _: form_source::HelloParams,
        mut r: form_source::HelloResults,
    ) -> Reply {
        let mut out = r.get();
        out.set_level(self.level); // its OWN level (a plain number); never the constant
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
        // limit 0 = the service's default page size; a larger limit is capped to the service's own maximum.
        let limit = match p.get_limit() {
            0 => 100,
            n => (n as usize).min(1000),
        };
        let cards = self.sh.cards.borrow();
        let end = skip.saturating_add(limit).min(cards.len());
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
                let due = if self.mode == Mode::LaxShape {
                    "2026-11-30T12:00:00-04[America/New_York]"
                } else {
                    "2026-11-30T12:00:00+01:00"
                }
                .to_owned();
                let mut cols = vec![("title", title), ("column", col)];
                if id == "c1" {
                    cols.push(("due", &due));
                }
                let mut values = view.init_values(u32::try_from(cols.len()).unwrap_or(0));
                for (j, (path, text)) in (0u32..).zip(cols) {
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
        p: form_source::BlobParams,
        mut r: form_source::BlobResults,
    ) -> Reply {
        if !self.blob {
            return Err(refuse("unsupported", 501, "the fake has no image or file field").error());
        }
        let p = p.get()?;
        if p.get_record()?.to_str()? != "c1" || p.get_field()?.to_str()? != "image" {
            return Err(refuse("unknown-record", 404, "no such image").error());
        }
        r.get().set_blob(capnp_rpc::new_client(Image));
        Ok(())
    }

    // Level 1 write: it does NOT go through the level 2 log (only `invokeIf` does), so `changes` never shows it.
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
                let lax = self.mode == Mode::LaxShape;
                out.set_failed(true);
                if lax {
                    out.reborrow().init_result().set_record("c1");
                }
                let mut prob: problem::Builder<'_> = out.init_problem();
                if lax {
                    prob.set_type(format!("https://freesynergy.net/problems/{}", why.kind));
                    prob.set_title(why.kind.as_str());
                } else {
                    prob.set_type(why.kind.as_str());
                    prob.set_title(why.title.as_str());
                }
                prob.set_detail(why.detail.as_str());
                prob.set_status(why.status);
                Ok(())
            }
        }
    }

    async fn invoke_if(
        self: Rc<Self>,
        p: form_source::InvokeIfParams,
        mut r: form_source::InvokeIfResults,
    ) -> Reply {
        self.need_level2()?;
        let p = p.get()?;
        let (command, record) = (p.get_command()?.to_str()?, p.get_record()?.to_str()?);
        let args: Vec<(String, String)> = p
            .get_args()?
            .iter()
            .map(|v| Ok((v.get_path()?.to_string()?, v.get_text()?.to_string()?)))
            .collect::<capnp::Result<_>>()?;
        let step = p.get_step()?;
        let (run, key) = (step.get_run()?.to_str()?, step.get_step()?.to_str()?);
        let entry = if let Some(done) = self.sh.replay(self.mode, false, run, key) {
            Outcome::Done(done)
        } else {
            let who = self.who.borrow().clone();
            self.sh.write_if(
                || self.run(command, record, &args),
                record,
                &args,
                p.get_expect()?.to_str()?,
                &who,
                (run, key),
            )
        };
        self.answer(&mut r, entry).await
    }

    async fn revert(
        self: Rc<Self>,
        p: form_source::RevertParams,
        mut r: form_source::RevertResults,
    ) -> Reply {
        self.need_level2()?;
        let p = p.get()?;
        let id = fake_l2_wire::uuid_of(p.get_change()?);
        let step = p.get_step()?;
        let (run, key) = (step.get_run()?.to_str()?, step.get_step()?.to_str()?);
        let entry = if let Some(done) = self.sh.replay(self.mode, true, run, key) {
            Outcome::Done(done)
        } else {
            let who = self.who.borrow().clone();
            self.sh.revert(id, &who, (run, key))
        };
        self.answer(&mut r, entry).await
    }

    async fn changes(
        self: Rc<Self>,
        p: form_source::ChangesParams,
        mut r: form_source::ChangesResults,
    ) -> Reply {
        self.need_level2()?;
        let p = p.get()?;
        if !p.get_scope()?.to_str()?.is_empty() {
            return Err(refuse("bad-arguments", 400, "the fake has no scopes").error());
        }
        let skip = match p.get_cursor()?.get_opaque()? {
            [] => 0,
            b => std::str::from_utf8(b)
                .ok()
                .and_then(|t| t.parse::<usize>().ok())
                .ok_or_else(|| refuse("bad-arguments", 400, "bad cursor").error())?,
        };
        let limit = match p.get_limit() {
            0 => 100,
            n => (n as usize).min(1000),
        };
        let program = self.who.borrow().program.clone();
        let all = self.sh.changes(p.get_run()?.to_str()?, &program);
        let end = skip.saturating_add(limit).min(all.len());
        let rows = all.get(skip..end).unwrap_or_default();
        let mut out = r.get();
        let mut list = out
            .reborrow()
            .init_items(u32::try_from(rows.len()).unwrap_or(0));
        for (i, e) in (0u32..).zip(rows) {
            fake_l2_wire::fill_change(list.reborrow().get(i), e);
        }
        if end < all.len() {
            out.init_next().set_opaque(end.to_string().as_bytes());
        }
        Ok(())
    }

    async fn introduce(
        self: Rc<Self>,
        p: form_source::IntroduceParams,
        _: form_source::IntroduceResults,
    ) -> Reply {
        self.need_level2()?;
        let p = p.get()?;
        *self.who.borrow_mut() = fake_l2::Who {
            program: p.get_program()?.to_string()?,
            instance: p.get_instance()?.to_string()?,
            device: p.get_device()?.to_string()?,
        };
        Ok(())
    }

    async fn watch(
        self: Rc<Self>,
        p: form_source::WatchParams,
        mut r: form_source::WatchResults,
    ) -> Reply {
        self.need_level2()?;
        let p = p.get()?;
        let form = p.get_form()?.to_str()?;
        let known = parse_key(form)
            .is_some_and(|(n, v)| n == "cards-in-columns" && (v.major, v.minor) == (1, 0));
        if !known {
            return Err(refuse("unknown-form", 404, "no such form").error());
        }
        let id = self.sh.watch(crate::fake::FORM, p.get_sink()?);
        r.get().set_subscription(capnp_rpc::new_client(Sub {
            sh: Rc::clone(&self.sh),
            id,
        }));
        Ok(())
    }
}
