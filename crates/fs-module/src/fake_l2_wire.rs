//! The wire side of level 2 of the fake: writing `ChangeRef` and `Conflict`, the subscription of `watch`, the shared answer of `invokeIf`/`revert`.
//!
//! Pattern: Fake Object (see `fake_l2`; the capnp builders and the server helpers in one place).

use std::rc::Rc;

use fs_interfaces::common_capnp::subscription;
use fs_interfaces::formsource_capnp::{change_ref, conflict, form_source, origin};

use crate::fake::{FakeSource, Refusal, refuse};
use crate::fake_l2::{self, Entry, Field, Outcome};

type Reply = Result<(), capnp::Error>;

/// The person the fake says for every connection (a UUIDv7 as IRI): the connection tells the person, never the caller.
const PERSON: &str = "urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e0f";

pub(crate) fn fill_change(mut out: change_ref::Builder<'_>, e: &Entry) {
    set_uuid(out.reborrow().init_activity(), e.id);
    out.set_object(e.object.as_str());
    out.set_revision(e.rev.to_string().as_str());
    out.set_before(
        e.before_rev
            .map(|r| r.to_string())
            .unwrap_or_default()
            .as_str(),
    );
    let mut o: origin::Builder<'_> = out.reborrow().init_origin();
    o.set_person(PERSON);
    o.set_program(e.who.program.as_str());
    o.set_instance(e.who.instance.as_str());
    o.set_device(e.who.device.as_str());
    out.set_kind(e.kind);
    out.reborrow().init_at().set_unix_millis(e.at);
    if e.undoes != 0 {
        set_uuid(out.init_undoes(), e.undoes);
    }
}

pub(crate) fn fill_conflict(mut out: conflict::Builder<'_>, by: Option<&Entry>, fields: &[Field]) {
    if let Some(e) = by {
        fill_change(out.reborrow().init_by(), e);
    }
    let mut list = out.init_fields(u32::try_from(fields.len()).unwrap_or(0));
    for (i, (p, cur, mine)) in (0u32..).zip(fields) {
        let mut f = list.reborrow().get(i);
        f.set_path(p.as_str());
        f.set_current(cur.as_str());
        f.set_mine(mine.as_str());
    }
}

fn set_uuid(mut u: fs_interfaces::common_capnp::uuid::Builder<'_>, id: u128) {
    u.set_hi(u64::try_from(id >> 64).unwrap_or(0));
    u.set_lo(u64::try_from(id & u128::from(u64::MAX)).unwrap_or(0));
}

pub(crate) fn uuid_of(u: fs_interfaces::common_capnp::uuid::Reader<'_>) -> u128 {
    (u128::from(u.get_hi()) << 64) | u128::from(u.get_lo())
}
/// The subscription of `watch`: dropping it ends it.
pub(crate) struct Sub {
    pub(crate) sh: Rc<fake_l2::Shared>,
    pub(crate) id: u64,
}

impl subscription::Server for Sub {}

impl Drop for Sub {
    fn drop(&mut self) {
        self.sh.unwatch(self.id);
    }
}

impl FakeSource {
    pub(crate) fn need_level2(&self) -> Result<(), capnp::Error> {
        if self.level < 2 {
            return Err(capnp::Error::unimplemented(
                "level 1: no level 2 method".into(),
            ));
        }
        Ok(())
    }

    /// The answer of `invokeIf`/`revert` (the same four fields) and then the push to the watchers.
    pub(crate) async fn answer<R: L2Answer>(&self, out: &mut R, outcome: Outcome) -> Reply {
        match outcome {
            Outcome::Done(e) => {
                out.done(&e);
                for sink in self.sh.sinks() {
                    let mut req = sink.changed_request();
                    fill_change(req.get().init_change(), &e);
                    req.send().await?;
                }
            }
            Outcome::Refused(why) => out.refuse(&why, None),
            Outcome::Clash { by, fields } => {
                out.refuse(
                    &refuse("conflict", 409, "changed since"),
                    Some((by, fields)),
                );
            }
        }
        Ok(())
    }
}

/// What `invokeIf` and `revert` answer alike.
pub(crate) trait L2Answer {
    fn done(&mut self, e: &fake_l2::Entry);
    fn refuse(
        &mut self,
        why: &Refusal,
        clash: Option<(Option<fake_l2::Entry>, Vec<fake_l2::Field>)>,
    );
}

macro_rules! l2_answer {
    ($t:ty) => {
        impl L2Answer for $t {
            fn done(&mut self, e: &fake_l2::Entry) {
                let mut made = self.get().init_result();
                made.set_record(if e.after.is_some() {
                    e.object.as_str()
                } else {
                    ""
                });
                made.set_text(e.text);
                fill_change(made.init_change(), e);
            }
            fn refuse(
                &mut self,
                why: &Refusal,
                clash: Option<(Option<fake_l2::Entry>, Vec<fake_l2::Field>)>,
            ) {
                let mut out = self.get();
                out.set_failed(true);
                if let Some((by, fields)) = clash {
                    fill_conflict(out.reborrow().init_conflict(), by.as_ref(), &fields);
                }
                let mut prob = out.init_problem();
                prob.set_type(why.kind.as_str());
                prob.set_title(why.title.as_str());
                prob.set_detail(why.detail.as_str());
                prob.set_status(why.status);
            }
        }
    };
}
l2_answer!(form_source::InvokeIfResults);
l2_answer!(form_source::RevertResults);
