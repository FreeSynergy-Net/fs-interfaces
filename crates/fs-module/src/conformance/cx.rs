//! Report, context and the small client helpers the rules share.
//!
//! Pattern: Test helper (plain functions; a setup that fails is a failure of the rule, never a panic).

use std::cell::RefCell;

use fs_interfaces::formsource_capnp::form_source;

use crate::FormSourceFixture;

/// The failed and the skipped rules of a run; empty `failures` means conformant.
#[derive(Debug, Default)]
pub struct Report {
    /// Rules that failed.
    pub failures: Vec<String>,
    /// Rules a fixture could not set up; not a failure, but not proven either.
    pub skipped: Vec<String>,
    /// How many checks ran.
    pub checked: usize,
}

pub(crate) struct Cx<'a> {
    pub fx: &'a dyn FormSourceFixture,
    pub src: form_source::Client,
    pub report: Report,
    /// Every refusal seen so far (an exception text or an answer); rule 14 checks the shape of all of them.
    pub seen: RefCell<Vec<Seen>>,
}

/// A refusal as it came in: an RPC exception (its text) or an `invoke` answer with `failed = true`.
#[derive(Debug)]
pub(crate) enum Seen {
    Exception(String),
    Answer(Answer),
}

/// One record of a page.
#[derive(Debug, Clone, Default)]
pub(crate) struct Row {
    pub id: String,
    pub may: Vec<String>,
    pub values: Vec<(String, String)>,
    pub hidden: Vec<String>,
}

#[derive(Debug, Default)]
pub(crate) struct Page {
    pub rows: Vec<Row>,
    pub total: u64,
    pub has_total: bool,
    pub set_may: Vec<String>,
    pub next: Vec<u8>,
}

#[derive(Debug, Default)]
pub(crate) struct Answer {
    pub failed: bool,
    pub kind: String,
    pub title: String,
    pub status: u16,
    pub record: String,
    pub text: String,
}

impl<'a> Cx<'a> {
    pub(crate) fn new(fx: &'a dyn FormSourceFixture) -> Self {
        Self {
            fx,
            src: fx.client(),
            report: Report::default(),
            seen: RefCell::default(),
        }
    }

    pub(crate) fn ensure(&mut self, ok: bool, rule: &str) {
        self.report.checked += 1;
        if !ok {
            self.report.failures.push(rule.to_owned());
        }
    }

    /// An RPC exception that names the problem kind `form-source-<kind>` as a whole word (not a part of a longer name).
    pub(crate) fn names(&self, e: &capnp::Result<impl Sized>, kind: &str) -> bool {
        let Some(text) = e.as_ref().err().map(ToString::to_string) else {
            return false;
        };
        self.seen.borrow_mut().push(Seen::Exception(text.clone()));
        has_word(&text, &format!("form-source-{kind}"))
    }

    pub(crate) fn skip(&mut self, rule: &str) {
        self.report.skipped.push(rule.to_owned());
    }

    pub(crate) async fn page(
        &self,
        form: &str,
        scope: &str,
        cursor: &[u8],
        limit: u32,
    ) -> capnp::Result<Page> {
        let mut req = self.src.page_request();
        {
            let mut p = req.get();
            p.set_form(form);
            p.set_scope(scope);
            p.set_limit(limit);
            p.init_cursor().set_opaque(cursor);
        }
        let answer = req.send().promise.await?;
        let a = answer.get()?;
        let pg = a.get_page()?;
        let texts = |l: capnp::text_list::Reader<'_>| {
            l.iter()
                .map(|t| Ok(t?.to_string()?))
                .collect::<capnp::Result<Vec<_>>>()
        };
        let mut rows = Vec::new();
        for r in pg.get_records()? {
            let values = r
                .get_values()?
                .iter()
                .map(|v| Ok((v.get_path()?.to_string()?, v.get_text()?.to_string()?)))
                .collect::<capnp::Result<_>>()?;
            let hidden = r
                .get_hidden()?
                .iter()
                .map(|h| Ok(h.get_path()?.to_string()?))
                .collect::<capnp::Result<_>>()?;
            rows.push(Row {
                id: r.get_id()?.to_string()?,
                may: texts(r.get_may()?)?,
                values,
                hidden,
            });
        }
        Ok(Page {
            rows,
            total: pg.get_total(),
            has_total: pg.get_has_total(),
            set_may: texts(pg.get_may()?)?,
            next: a.get_next()?.get_opaque()?.to_vec(),
        })
    }

    /// All records of a form, by walking with `limit` (guarded).
    pub(crate) async fn walk(
        &self,
        form: &str,
        scope: &str,
        limit: u32,
    ) -> capnp::Result<Vec<Row>> {
        let (mut rows, mut cursor) = (Vec::new(), Vec::new());
        for _ in 0..10_000 {
            let p = self.page(form, scope, &cursor, limit).await?;
            if p.rows.is_empty() && !p.next.is_empty() {
                return Err(capnp::Error::failed(
                    "an empty page with a next cursor".into(),
                ));
            }
            rows.extend(p.rows);
            if p.next.is_empty() {
                return Ok(rows);
            }
            cursor = p.next;
        }
        Err(capnp::Error::failed("the cursor never ends".into()))
    }

    pub(crate) async fn invoke(
        &self,
        command: &str,
        record: &str,
        args: &[(&str, &str)],
    ) -> capnp::Result<Answer> {
        let mut req = self.src.invoke_request();
        {
            let mut p = req.get();
            p.set_command(command);
            p.set_record(record);
            let mut list = p.init_args(u32::try_from(args.len()).unwrap_or(0));
            for (i, (k, v)) in (0u32..).zip(args) {
                let mut a = list.reborrow().get(i);
                a.set_path(k);
                a.set_text(v);
            }
        }
        let answer = req.send().promise.await?;
        let a = answer.get()?;
        let prob = a.get_problem()?;
        let answer = Answer {
            failed: a.get_failed(),
            kind: prob.get_type()?.to_string()?,
            title: prob.get_title()?.to_string()?,
            status: prob.get_status(),
            record: a.get_result()?.get_record()?.to_string()?,
            text: a.get_result()?.get_text()?.to_string()?,
        };
        if answer.failed {
            self.seen.borrow_mut().push(Seen::Answer(Answer {
                kind: answer.kind.clone(),
                title: answer.title.clone(),
                record: answer.record.clone(),
                text: answer.text.clone(),
                ..answer
            }));
        }
        Ok(answer)
    }
}

/// `word` stands in `text` not as the start of a longer name (`form-source-unknown-form-x` is not `form-source-unknown-form`).
pub(crate) fn has_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(i, _)| {
        !text[i + word.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-')
    })
}

pub(crate) const NOWHERE: &str = "zz-no-such";

pub(crate) fn refused(a: &Answer, kind: &str, status: u16) -> bool {
    a.failed && a.kind == format!("form-source-{kind}") && a.status == status
}

pub(crate) async fn first_record(cx: &Cx<'_>) -> Option<String> {
    let f = cx.fx.forms().into_iter().next()?;
    cx.page(&f.key, &f.scope, &[], 1)
        .await
        .ok()?
        .rows
        .into_iter()
        .next()
        .map(|r| r.id)
}

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
