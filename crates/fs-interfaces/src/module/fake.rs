//! A fake module "only the owner may change" (feature `fake`): proves the contract can be implemented and gives the wrapper tests a subject.
//!
//! Pattern: Strategy (test variant of [`RightsRules`]) in a Template Method subject ([`ModuleCore`]). Deviation: the fake keeps its
//! records in memory and writes no change log; it counts the calls of `apply` so a test can show that a refused call never got there.

#![allow(clippy::unused_async_trait_impl, clippy::unnecessary_literal_bound)] // the fake has nothing to await; the trait is async

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use fs_types::PersonRef;

use super::ModuleCore;
use super::{
    Applied, Ask, Blob, ChangeFeed, ChangePage, ChangesQuery, Context, HiddenReason, Invocation,
    Page, PageQuery, RecordView, Refusal, RightsRules, StepRef, Value, Verdict,
};

type Owners = Arc<Mutex<HashMap<String, PersonRef>>>;

/// The command that makes a record; the maker is its owner.
pub const CREATE: &str = "note.create";
/// The command that changes a record; only its owner may run it.
pub const SET: &str = "note.set";

/// The rules: a create is free, every other command only for the owner of the record.
#[derive(Clone, Debug, Default)]
pub struct OwnerOnly {
    owners: Owners,
}

impl RightsRules for OwnerOnly {
    fn may(&self, ask: &Ask<'_>) -> Verdict {
        if ask.command == CREATE {
            return Verdict::Allow;
        }
        let owners = self.owners.lock().expect("owners");
        match ask.record.and_then(|r| owners.get(r)) {
            Some(owner) if *owner == ask.who.person => Verdict::Allow,
            Some(_) => Verdict::deny("not-owner"),
            None => Verdict::deny("unknown-record"),
        }
    }

    fn hidden(&self, _who: &Context, _record: &str, _field: &str) -> Option<HiddenReason> {
        None
    }
}

/// The fake module: records `id -> text`, owners in a table shared with its rules.
#[derive(Debug, Default)]
pub struct OwnerOnlyModule {
    rules: OwnerOnly,
    texts: Mutex<HashMap<String, String>>,
    applied: AtomicUsize,
}

impl OwnerOnlyModule {
    /// An empty module.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How often `apply` ran: a call the rules refused never counts.
    #[must_use]
    pub fn applied(&self) -> usize {
        self.applied.load(Ordering::SeqCst)
    }
}

impl ModuleCore for OwnerOnlyModule {
    fn level(&self) -> u32 {
        2
    }

    fn provider(&self) -> &str {
        "net.freesynergy.fake.owner-only"
    }

    fn rules(&self) -> &dyn RightsRules {
        &self.rules
    }

    async fn page(&self, _cx: &Context, _query: PageQuery) -> Result<Page, Refusal> {
        let texts = self.texts.lock().expect("texts");
        let mut records: Vec<RecordView> = texts
            .iter()
            .map(|(id, text)| RecordView {
                id: id.clone(),
                values: vec![Value {
                    path: "text".into(),
                    text: text.clone(),
                }],
                ..RecordView::default()
            })
            .collect();
        records.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(Page {
            total: Some(records.len() as u64),
            records,
            ..Page::default()
        })
    }

    async fn blob(&self, _cx: &Context, _record: &str, _field: &str) -> Result<Blob, Refusal> {
        Err(Refusal {
            kind: "unsupported".into(),
            status: 501,
            ..Refusal::default()
        })
    }

    async fn apply(&self, cx: &Context, command: Invocation) -> Result<Applied, Refusal> {
        self.applied.fetch_add(1, Ordering::SeqCst);
        let text = command
            .args
            .iter()
            .find(|v| v.path == "text")
            .map(|v| v.text.clone())
            .unwrap_or_default();
        let id = match command.command.as_str() {
            CREATE => {
                let id = format!("note-{}", self.texts.lock().expect("texts").len() + 1);
                self.rules
                    .owners
                    .lock()
                    .expect("owners")
                    .insert(id.clone(), cx.person.clone());
                id
            }
            SET => command.record.ok_or_else(|| Refusal {
                kind: "bad-arguments".into(),
                status: 400,
                ..Refusal::default()
            })?,
            _ => {
                return Err(Refusal {
                    kind: "unknown-command".into(),
                    status: 404,
                    ..Refusal::default()
                });
            }
        };
        self.texts.lock().expect("texts").insert(id.clone(), text);
        Ok(Applied {
            record: id,
            ..Applied::default()
        })
    }

    async fn revert(
        &self,
        _cx: &Context,
        _change: &str,
        _step: StepRef,
    ) -> Result<Applied, Refusal> {
        Err(Refusal {
            kind: "not-revertible".into(),
            status: 409,
            ..Refusal::default()
        })
    }

    async fn changes(&self, _cx: &Context, _query: ChangesQuery) -> Result<ChangePage, Refusal> {
        Ok(ChangePage::default())
    }

    fn feed(&self, _sink: ChangeFeed) {}
}
