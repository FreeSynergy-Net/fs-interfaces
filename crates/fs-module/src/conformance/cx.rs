//! Report, context and the small client helpers the rules share.
//!
//! Pattern: Test helper (plain functions; a setup that fails is a failure of the rule, never a panic).

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
}

/// One record of a page.
#[derive(Debug, Clone, Default)]
pub(crate) struct Row {
    pub id: String,
    pub may: Vec<String>,
    pub values: Vec<(String, String)>,
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
    pub status: u16,
    pub record: String,
}

impl<'a> Cx<'a> {
    pub(crate) fn new(fx: &'a dyn FormSourceFixture) -> Self {
        Self {
            fx,
            src: fx.client(),
            report: Report::default(),
        }
    }

    pub(crate) fn ensure(&mut self, ok: bool, rule: &str) {
        self.report.checked += 1;
        if !ok {
            self.report.failures.push(rule.to_owned());
        }
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
            rows.push(Row {
                id: r.get_id()?.to_string()?,
                may: texts(r.get_may()?)?,
                values,
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
        Ok(Answer {
            failed: a.get_failed(),
            kind: prob.get_type()?.to_string()?,
            status: prob.get_status(),
            record: a.get_result()?.get_record()?.to_string()?,
        })
    }
}

/// An RPC exception that names the kind `form-source-<kind>`.
pub(crate) fn names(e: &capnp::Result<impl Sized>, kind: &str) -> bool {
    e.as_ref()
        .err()
        .is_some_and(|e| e.to_string().contains(&format!("form-source-{kind}")))
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
