//! The `FormSource` server of the fake (the rules of level 1: errors in the answer at `invoke`, in an exception at `page`/`blob`).
//!
//! Pattern: Fake Object (the server side of the fake: the rules of level 1 in one place).
#![allow(clippy::unused_async_trait_impl)] // the generated server trait is async

use fs_interfaces::common_capnp::problem;
use fs_interfaces::formsource_capnp::{form_source, value};
use std::rc::Rc;

use crate::fake::{FakeSource, Mode, refuse};
use crate::fake_l2::{self, Outcome};
use crate::fake_l2_wire::{self, Sub};
use crate::fake_rows::{knows_form, window};
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
        out.set_provider(fs_interfaces::module::SAMPLE_PROVIDER);
        Ok(())
    }

    async fn page(
        self: Rc<Self>,
        p: form_source::PageParams,
        mut r: form_source::PageResults,
    ) -> Reply {
        let p = p.get()?;
        let page = self
            .rows(
                p.get_form()?.to_str()?,
                p.get_scope()?.to_str()?,
                p.get_cursor()?.get_opaque()?,
                p.get_limit(),
            )
            .map_err(|why| why.error())?;
        let mut out = r.get();
        {
            let mut view = out.reborrow().init_page();
            view.set_total(page.total as u64);
            view.set_has_total(true);
            view.reborrow().init_may(1).set(0, "card.create");
            let mut list = view.init_records(
                u32::try_from(page.rows.len())
                    .map_err(|_| capnp::Error::failed("too many".into()))?,
            );
            for (i, row) in (0u32..).zip(&page.rows) {
                let mut rec = list.reborrow().get(i);
                rec.set_id(row.id.as_str());
                if FakeSource::changeable(&row.id) {
                    let mut may = rec.reborrow().init_may(2);
                    may.set(0, "card.move");
                    may.set(1, "card.delete");
                }
                let mut values = rec.init_values(u32::try_from(row.values.len()).unwrap_or(0));
                for (j, (path, text)) in (0u32..).zip(&row.values) {
                    let mut v: value::Builder<'_> = values.reborrow().get(j);
                    v.set_path(path.as_str());
                    v.set_text(text.as_str());
                }
            }
        }
        if let Some(next) = &page.next {
            out.init_next().set_opaque(next.as_bytes());
        }
        Ok(())
    }

    async fn blob(
        self: Rc<Self>,
        p: form_source::BlobParams,
        mut r: form_source::BlobResults,
    ) -> Reply {
        let p = p.get()?;
        self.check_blob(p.get_record()?.to_str()?, p.get_field()?.to_str()?)
            .map_err(|why| why.error())?;
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
        self.check_scope(p.get_scope()?.to_str()?)?;
        let (skip, limit) =
            window(p.get_cursor()?.get_opaque()?, p.get_limit()).map_err(|why| why.error())?;
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
        if !knows_form(form) {
            return Err(refuse("unknown-form", 404, "no such form").error());
        }
        self.check_scope(p.get_scope()?.to_str()?)?;
        let id = self.sh.watch(crate::fake::FORM, p.get_sink()?);
        r.get().set_subscription(capnp_rpc::new_client(Sub {
            sh: Rc::clone(&self.sh),
            id,
        }));
        Ok(())
    }
}
