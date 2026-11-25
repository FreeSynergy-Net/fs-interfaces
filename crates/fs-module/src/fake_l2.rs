//! Level 2 of the fake: its own small log of changes in memory (no `fs-objects`), the shared state of all connections.
//!
//! Pattern: Fake Object (the level 2 rules in one place: compare-and-set, idempotent step, undo as a new change, push).
//! Deviation: the log is a `Vec`; a revision is a counter per record; a change is "unchanged" when it is the last one of its record.

use std::cell::RefCell;
use std::collections::HashMap;

use fs_interfaces::formsource_capnp::change_sink;
use fs_interfaces::module::ChangeFeed;

use crate::fake::{FORM, Mode, Refusal};

/// The level the fake itself speaks (a plain number; never the constant of the contract).
pub(crate) const OWN_LEVEL: u32 = 2;

/// What `introduce` said on one connection.
#[derive(Clone, Debug, Default)]
pub(crate) struct Who {
    pub program: String,
    pub instance: String,
    pub device: String,
}

/// A card as stored: title and column, and its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Img {
    pub title: String,
    pub column: String,
    pub at: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub id: u128,
    pub object: String,
    pub before_rev: Option<u64>,
    pub rev: u64,
    pub who: Who,
    pub kind: &'static str,
    pub at: i64,
    pub undoes: u128,
    pub run: String,
    pub text: &'static str,
    pub before: Option<Img>,
    pub after: Option<Img>,
}

struct Watcher {
    id: u64,
    form: String,
    sink: change_sink::Client,
}

#[derive(Default)]
struct Log {
    entries: Vec<Entry>,
    rev: HashMap<String, u64>,
    seq: u64,
    steps: HashMap<(bool, String, String), usize>,
    watchers: Vec<Watcher>,
    next_watch: u64,
}

/// The state all connections of one fake share: the cards and the log.
pub(crate) struct Shared {
    pub cards: RefCell<Vec<(String, String, String)>>,
    pub next: RefCell<u32>,
    log: RefCell<Log>,
    /// The plain callbacks of `ModuleCore::feed` (next to the capnp sinks in the log): `commit` tells them every new change.
    feeds: RefCell<Vec<ChangeFeed>>,
}

/// A clash field: path, what the record has now, what the caller wanted.
pub(crate) type Field = (String, String, String);

/// How a level 2 write ended: the entry (new or replayed), or the refusal.
pub(crate) enum Outcome {
    Done(Entry),
    Refused(Refusal),
    Clash {
        by: Option<Entry>,
        fields: Vec<(String, String, String)>,
    },
}

impl Shared {
    pub(crate) fn new() -> Self {
        let card =
            |id: &str, title: &str, col: &str| (id.to_owned(), title.to_owned(), col.to_owned());
        let log = Log {
            rev: ["c1", "c2", "c3"].map(|c| (c.to_owned(), 1)).into(),
            ..Log::default()
        };
        Self {
            cards: RefCell::new(vec![
                card("c1", "One", "todo"),
                card("c2", "Two", "todo"),
                card("c3", "Three", "done"),
            ]),
            next: RefCell::new(4),
            log: RefCell::new(log),
            feeds: RefCell::default(),
        }
    }

    pub(crate) fn image(&self, id: &str) -> Option<Img> {
        let cards = self.cards.borrow();
        let at = cards.iter().position(|c| c.0 == id)?;
        Some(Img {
            title: cards[at].1.clone(),
            column: cards[at].2.clone(),
            at,
        })
    }

    pub(crate) fn put(&self, id: &str, img: Option<&Img>) {
        let mut cards = self.cards.borrow_mut();
        let old = cards
            .iter()
            .position(|c| c.0 == id)
            .map(|i| cards.remove(i));
        if let Some(i) = img {
            let at = i.at.min(cards.len());
            cards.insert(at, (id.to_owned(), i.title.clone(), i.column.clone()));
        } else {
            drop(old);
        }
    }

    pub(crate) fn rev_of(&self, id: &str) -> Option<u64> {
        self.log.borrow().rev.get(id).copied()
    }

    pub(crate) fn find(&self, id: u128) -> Option<Entry> {
        self.log
            .borrow()
            .entries
            .iter()
            .find(|e| e.id == id)
            .cloned()
    }

    pub(crate) fn last_of(&self, object: &str) -> Option<Entry> {
        self.log
            .borrow()
            .entries
            .iter()
            .rev()
            .find(|e| e.object == object)
            .cloned()
    }

    /// A replay of the same step, unless the mode ignores steps.
    pub(crate) fn replay(&self, mode: Mode, revert: bool, run: &str, step: &str) -> Option<Entry> {
        if mode == Mode::IgnoreStep || step.is_empty() {
            return None;
        }
        let log = self.log.borrow();
        let at = *log.steps.get(&(revert, run.to_owned(), step.to_owned()))?;
        log.entries.get(at).cloned()
    }

    /// Writes one entry: moves the revision, remembers the step. Returns it for the push.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn commit(
        &self,
        object: &str,
        who: &Who,
        kind: &'static str,
        undoes: u128,
        key: (bool, &str, &str),
        text: &'static str,
        images: (Option<Img>, Option<Img>),
    ) -> Entry {
        let mut log = self.log.borrow_mut();
        log.seq += 1;
        let before_rev = log.rev.get(object).copied();
        let rev = before_rev.unwrap_or(0) + 1;
        log.rev.insert(object.to_owned(), rev);
        let at = 1_790_000_000_000 + i64::try_from(log.seq).unwrap_or(0) * 1000;
        // UUIDv7: 48 bits of time, version 7, variant 10, the counter in the low bits.
        let hi = (u128::try_from(at).unwrap_or(0) << 80) | (0x7_u128 << 76);
        let id = hi | (0b10_u128 << 62) | u128::from(log.seq);
        let entry = Entry {
            id,
            object: object.to_owned(),
            before_rev,
            rev,
            who: who.clone(),
            kind,
            at,
            undoes,
            run: key.1.to_owned(),
            text,
            before: images.0,
            after: images.1,
        };
        if !key.2.is_empty() {
            let n = log.entries.len();
            log.steps
                .insert((key.0, key.1.to_owned(), key.2.to_owned()), n);
        }
        log.entries.push(entry.clone());
        drop(log); // no borrow of the log while a callback runs
        let change = crate::fake_core::change_ref(&entry);
        for feed in self.feeds.borrow().iter() {
            feed(&change);
        }
        entry
    }

    /// Registers a plain callback for every change from now on (`ModuleCore::feed`).
    pub(crate) fn add_feed(&self, sink: ChangeFeed) {
        self.feeds.borrow_mut().push(sink);
    }

    /// The changes, newest first: those of a run, or those of a program.
    pub(crate) fn changes(&self, run: &str, program: &str) -> Vec<Entry> {
        let log = self.log.borrow();
        log.entries
            .iter()
            .rev()
            .filter(|e| {
                if run.is_empty() {
                    e.who.program == program
                } else {
                    e.run == run
                }
            })
            .cloned()
            .collect()
    }

    pub(crate) fn watch(&self, form: &str, sink: change_sink::Client) -> u64 {
        let mut log = self.log.borrow_mut();
        log.next_watch += 1;
        let id = log.next_watch;
        log.watchers.push(Watcher {
            id,
            form: form.to_owned(),
            sink,
        });
        id
    }

    pub(crate) fn unwatch(&self, id: u64) {
        self.log.borrow_mut().watchers.retain(|w| w.id != id);
    }

    /// The sinks to tell about a new change (taken out, so nothing stays borrowed across an `await`).
    pub(crate) fn sinks(&self) -> Vec<change_sink::Client> {
        let log = self.log.borrow();
        log.watchers
            .iter()
            .filter(|w| w.form == FORM)
            .map(|w| w.sink.clone())
            .collect()
    }
}
