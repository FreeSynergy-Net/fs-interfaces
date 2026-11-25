//! `FakeCore`: the fake as a `ModuleCore` (no capnp), and `FakeRules`, its rights. The effect lives in the module, the right in the rules.
//!
//! Pattern: Fake Object (Test Double) as `ModuleCore`; deviation: it checks no rights, they are in `FakeRules` (a wrapper asks them).
//! It uses the same facts as `FakeSource` (`Shared`, `effect`, `rows`), so there is ONE body of the fake.

#![allow(clippy::unused_async_trait_impl, clippy::unnecessary_literal_bound)] // the fake has nothing to await; the trait is async
#![allow(clippy::result_large_err)] // `module::Refusal` is the error type of the contract `ModuleCore`

use fs_interfaces::module::{
    Applied, Ask, Blob, ChangeFeed, ChangeKind, ChangePage, ChangeRef, ChangesQuery, Conflict,
    Context, FieldClash, HiddenReason, Invocation, ModuleCore, Origin, Page, PageQuery, RecordView,
    RightsRules, StepRef, Value, Verdict,
};
use fs_types::Utc;

use crate::fake::{FakeSource, Mode, Refusal};
use crate::fake_l2::{Entry, Outcome, Who};
use crate::fake_l2_wire::PERSON;
use crate::fake_rows::window;

/// Pattern: Strategy (test variant of `RightsRules`) - the rights of the fake: the locked card `c3` and the set command `card.clear` are
/// denied (`locked`), everything else is allowed, nothing is hidden. The rule itself is `FakeSource::locked`, the same the level 1 server asks.
#[derive(Clone, Copy, Debug, Default)]
pub struct FakeRules;

impl RightsRules for FakeRules {
    fn may(&self, ask: &Ask<'_>) -> Verdict {
        match ask.command {
            "change.revert" | "change.watch" => Verdict::Allow,
            c if FakeSource::locked(c, ask.record.unwrap_or_default()) => Verdict::deny("locked"),
            _ => Verdict::Allow,
        }
    }

    fn hidden(&self, _who: &Context, _record: &str, _field: &str) -> Option<HiddenReason> {
        None
    }
}

/// Pattern: Fake Object (Test Double) as `ModuleCore`; deviation: it prüft keine Rechte (they are in [`FakeRules`]), it only acts.
/// The same small board as [`FakeSource`] (same state, same modes), level 2, provider `fake`, without capnp.
pub struct FakeCore {
    src: FakeSource,
    rules: FakeRules,
}

impl FakeCore {
    /// The fake with three cards, `mode` as at [`FakeSource::new`] (a mode breaks one rule on purpose, `IgnoreStep` the step rule).
    #[must_use]
    pub fn new(mode: Mode) -> Self {
        Self {
            src: FakeSource::new(mode),
            rules: FakeRules,
        }
    }

    /// The same, with an image field (`image` of card `c1`).
    #[must_use]
    pub fn with_blob(mut self) -> Self {
        self.src = self.src.with_blob();
        self
    }

    /// The same, with `n` boards (default 1).
    #[must_use]
    pub fn with_boards(mut self, n: usize) -> Self {
        self.src = self.src.with_boards(n);
        self
    }
}

/// The text of a UUID (8-4-4-4-12, lower case): the form a change id has as text.
fn uuid_text(id: u128) -> String {
    let h = format!("{id:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}

/// The number behind a UUID text; `None` if it is none.
fn uuid_number(text: &str) -> Option<u128> {
    let h: String = text.chars().filter(|c| *c != '-').collect();
    (h.len() == 32)
        .then(|| u128::from_str_radix(&h, 16).ok())
        .flatten()
}

/// A log entry as a `ChangeRef` (the person is the one the fake says for every connection).
pub(crate) fn change_ref(e: &Entry) -> ChangeRef {
    let kind = match e.kind {
        "Create" => ChangeKind::Create,
        "Delete" => ChangeKind::Delete,
        "Undo" => ChangeKind::Undo,
        _ => ChangeKind::Update,
    };
    let c = ChangeRef::new(
        &uuid_text(e.id),
        &e.object,
        &e.rev.to_string(),
        &e.before_rev.map(|r| r.to_string()).unwrap_or_default(),
        Origin::new(PERSON, &e.who.program, &e.who.instance, &e.who.device),
        kind,
        Utc::from_millis(e.at),
    );
    if e.undoes == 0 {
        c
    } else {
        c.undoing(&uuid_text(e.undoes))
    }
}

/// The refusal of the fake as the value of the module contract (`form-source-` is the problem type, the module says the bare kind).
fn refusal(why: &Refusal) -> fs_interfaces::module::Refusal {
    let mut r = fs_interfaces::module::Refusal::default();
    why.kind
        .strip_prefix("form-source-")
        .unwrap_or(&why.kind)
        .clone_into(&mut r.kind);
    r.status = why.status;
    r.detail.clone_from(&why.detail);
    r
}

fn who_of(cx: &Context) -> Who {
    Who {
        program: cx.claimed.program.clone(),
        instance: cx.claimed.instance.clone(),
        device: cx.claimed.device.clone(),
    }
}

fn applied(e: &Entry) -> Applied {
    let mut a = Applied::default();
    if e.after.is_some() {
        a.record.clone_from(&e.object);
    }
    e.text.clone_into(&mut a.text);
    a.change = Some(change_ref(e));
    a
}

fn outcome(out: Outcome) -> Result<Applied, fs_interfaces::module::Refusal> {
    match out {
        Outcome::Done(e) => Ok(applied(&e)),
        Outcome::Refused(why) => Err(refusal(&why)),
        Outcome::Clash { by, fields } => {
            let mut r = refusal(&crate::fake::refuse("conflict", 409, "changed since"));
            r.kind = "conflict".into();
            if let Some(by) = by {
                let fields = fields
                    .iter()
                    .map(|(p, c, m)| FieldClash::new(p, c, m))
                    .collect();
                r = fs_interfaces::module::Refusal::conflict(
                    &r.detail,
                    Conflict::new(change_ref(&by), fields),
                );
            }
            Err(r)
        }
    }
}

impl ModuleCore for FakeCore {
    fn level(&self) -> u32 {
        crate::fake_l2::OWN_LEVEL
    }

    fn provider(&self) -> &str {
        "fake"
    }

    fn rules(&self) -> &dyn RightsRules {
        &self.rules
    }

    async fn page(
        &self,
        _cx: &Context,
        q: PageQuery,
    ) -> Result<Page, fs_interfaces::module::Refusal> {
        let rows = self
            .src
            .rows(&q.form, &q.scope, q.cursor.as_bytes(), q.limit)
            .map_err(|why| refusal(&why))?;
        let mut page = Page::default();
        page.records = rows
            .rows
            .into_iter()
            .map(|row| {
                let mut v = RecordView::default();
                v.id = row.id;
                v.values = row.values.iter().map(|(p, t)| Value::new(p, t)).collect();
                v
            })
            .collect();
        page.total = Some(rows.total as u64);
        page.next = rows.next.unwrap_or_default();
        Ok(page)
    }

    async fn blob(
        &self,
        _cx: &Context,
        record: &str,
        field: &str,
    ) -> Result<Blob, fs_interfaces::module::Refusal> {
        self.src
            .check_blob(record, field)
            .map_err(|why| refusal(&why))?;
        let mut b = Blob::default();
        b.media_type = "image/png".into();
        b.bytes = crate::fake::IMAGE.to_vec();
        Ok(b)
    }

    async fn apply(
        &self,
        cx: &Context,
        c: Invocation,
    ) -> Result<Applied, fs_interfaces::module::Refusal> {
        let record = c.record.unwrap_or_default();
        let args: Vec<(String, String)> = c
            .args
            .iter()
            .map(|v| (v.path.clone(), v.text.clone()))
            .collect();
        let step = c.step.unwrap_or_default();
        let who = who_of(cx);
        let mode = self.src.mode;
        let out = if let Some(done) = self.src.sh.replay(mode, false, &step.run, &step.step) {
            Outcome::Done(done)
        } else {
            self.src.sh.write_if(
                || self.src.effect(&c.command, &record, &args),
                &record,
                &args,
                c.expect.as_deref().unwrap_or_default(),
                &who,
                (&step.run, &step.step),
            )
        };
        outcome(out)
    }

    async fn revert(
        &self,
        cx: &Context,
        change: &str,
        step: StepRef,
    ) -> Result<Applied, fs_interfaces::module::Refusal> {
        let Some(id) = uuid_number(change) else {
            return Err(refusal(&crate::fake::refuse(
                "unknown-change",
                404,
                "no such change",
            )));
        };
        let out = if let Some(done) = self
            .src
            .sh
            .replay(self.src.mode, true, &step.run, &step.step)
        {
            Outcome::Done(done)
        } else {
            self.src.sh.revert(id, &who_of(cx), (&step.run, &step.step))
        };
        outcome(out)
    }

    async fn changes(
        &self,
        cx: &Context,
        q: ChangesQuery,
    ) -> Result<ChangePage, fs_interfaces::module::Refusal> {
        let (skip, limit) = window(q.cursor.as_bytes(), q.limit).map_err(|why| refusal(&why))?;
        let all = self.src.sh.changes(&q.run, &cx.claimed.program);
        let end = skip.saturating_add(limit).min(all.len());
        let mut page = ChangePage::default();
        page.changes = all
            .get(skip..end)
            .unwrap_or_default()
            .iter()
            .map(change_ref)
            .collect();
        if end < all.len() {
            page.next = end.to_string();
        }
        Ok(page)
    }

    fn feed(&self, sink: ChangeFeed) {
        self.src.sh.add_feed(sink);
    }
}
