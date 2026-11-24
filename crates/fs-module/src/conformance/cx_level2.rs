//! The calls of level 2 for the rules: `invokeIf`, `revert`, `changes`, `introduce` and a receiver for `watch`, on any connection.
//!
//! Pattern: Test helper (plain functions; see `cx`).

use std::cell::RefCell;

use fs_interfaces::formsource_capnp::form_source;

/// A change as the module names it (`ChangeRef`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Change {
    pub activity: u128,
    pub object: String,
    pub revision: String,
    pub before: String,
    pub program: String,
    pub kind: String,
    pub undoes: u128,
}

/// The answer of `invokeIf` and `revert`.
#[derive(Debug, Default)]
pub(crate) struct IfAnswer {
    pub failed: bool,
    pub kind: String,
    pub status: u16,
    pub change: Option<Change>,
    pub by: Option<Change>,
}

pub(crate) fn read_change(
    c: fs_interfaces::formsource_capnp::change_ref::Reader<'_>,
) -> capnp::Result<Change> {
    let uuid = |u: fs_interfaces::common_capnp::uuid::Reader<'_>| {
        (u128::from(u.get_hi()) << 64) | u128::from(u.get_lo())
    };
    Ok(Change {
        activity: uuid(c.get_activity()?),
        object: c.get_object()?.to_string()?,
        revision: c.get_revision()?.to_string()?,
        before: c.get_before()?.to_string()?,
        program: c.get_origin()?.get_program()?.to_string()?,
        kind: c.get_kind()?.to_string()?,
        undoes: uuid(c.get_undoes()?),
    })
}

/// A UUIDv7: version 7 and variant `10`.
pub(crate) fn is_v7(id: u128) -> bool {
    (id >> 76) & 0xf == 7 && (id >> 62) & 0b11 == 0b10
}

fn put_uuid(mut u: fs_interfaces::common_capnp::uuid::Builder<'_>, id: u128) {
    u.set_hi(u64::try_from(id >> 64).unwrap_or(0));
    u.set_lo(u64::try_from(id & u128::from(u64::MAX)).unwrap_or(0));
}

fn read_if(
    failed: bool,
    p: fs_interfaces::common_capnp::problem::Reader<'_>,
    w: fs_interfaces::formsource_capnp::write_result::Reader<'_>,
    by: Option<fs_interfaces::formsource_capnp::change_ref::Reader<'_>>,
) -> capnp::Result<IfAnswer> {
    Ok(IfAnswer {
        failed,
        kind: p.get_type()?.to_string()?,
        status: p.get_status(),
        change: if w.has_change() {
            Some(read_change(w.get_change()?)?)
        } else {
            None
        },
        by: by.map(read_change).transpose()?,
    })
}

/// `invokeIf` on any connection.
pub(crate) async fn invoke_if(
    src: &form_source::Client,
    (command, record, args): (&str, &str, &[(String, String)]),
    expect: &str,
    (run, step): (&str, &str),
) -> capnp::Result<IfAnswer> {
    let mut req = src.invoke_if_request();
    {
        let mut p = req.get();
        p.set_command(command);
        p.set_record(record);
        p.set_expect(expect);
        let mut s = p.reborrow().init_step();
        s.set_run(run);
        s.set_step(step);
        let mut list = p.init_args(u32::try_from(args.len()).unwrap_or(0));
        for (i, (k, v)) in (0u32..).zip(args) {
            let mut a = list.reborrow().get(i);
            a.set_path(k);
            a.set_text(v);
        }
    }
    let answer = req.send().promise.await?;
    let a = answer.get()?;
    let by = a.get_conflict()?;
    read_if(
        a.get_failed(),
        a.get_problem()?,
        a.get_result()?,
        by.has_by().then(|| by.get_by()).transpose()?,
    )
}

/// `revert` on any connection.
pub(crate) async fn revert(
    src: &form_source::Client,
    change: u128,
    (run, step): (&str, &str),
) -> capnp::Result<IfAnswer> {
    let mut req = src.revert_request();
    {
        let mut p = req.get();
        put_uuid(p.reborrow().init_change(), change);
        let mut s = p.init_step();
        s.set_run(run);
        s.set_step(step);
    }
    let answer = req.send().promise.await?;
    let a = answer.get()?;
    let by = a.get_conflict()?;
    read_if(
        a.get_failed(),
        a.get_problem()?,
        a.get_result()?,
        by.has_by().then(|| by.get_by()).transpose()?,
    )
}

/// `changes` on any connection (one page of up to 100).
pub(crate) async fn changes(src: &form_source::Client, run: &str) -> capnp::Result<Vec<Change>> {
    let mut req = src.changes_request();
    {
        let mut p = req.get();
        p.set_run(run);
        p.set_limit(100);
    }
    let answer = req.send().promise.await?;
    answer.get()?.get_items()?.iter().map(read_change).collect()
}

/// `introduce` on any connection.
pub(crate) async fn introduce(
    src: &form_source::Client,
    program: &str,
    instance: &str,
    device: &str,
) -> capnp::Result<()> {
    let mut req = src.introduce_request();
    {
        let mut p = req.get();
        p.set_program(program);
        p.set_instance(instance);
        p.set_device(device);
    }
    req.send().promise.await?;
    Ok(())
}

/// A receiver of `watch` that keeps what it hears.
pub(crate) struct Heard(pub std::rc::Rc<RefCell<Vec<Change>>>);

#[allow(clippy::unused_async_trait_impl)] // the generated server trait is async
impl fs_interfaces::formsource_capnp::change_sink::Server for Heard {
    async fn changed(
        self: std::rc::Rc<Self>,
        p: fs_interfaces::formsource_capnp::change_sink::ChangedParams,
    ) -> Result<(), capnp::Error> {
        self.0
            .borrow_mut()
            .push(read_change(p.get()?.get_change()?)?);
        Ok(())
    }
}
