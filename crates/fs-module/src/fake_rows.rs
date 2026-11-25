//! The reading side of the fake without any capnp: the rows of a page, the window of a cursor, the check of a `blob` call.
//!
//! Pattern: Fake Object (see `fake_server`; the part of `page`/`blob` that both the wire server `FakeSource` and `FakeCore` use).

use fs_interfaces::forms::parse_key;

use crate::fake::{FakeSource, LOCKED, Mode, Refusal, refuse};
use crate::fake_values::standard_values;

/// One card as read: its id and its values (path, text).
#[derive(Clone, Debug)]
pub(crate) struct Row {
    pub id: String,
    pub values: Vec<(String, String)>,
}

/// One page of rows with the total and the cursor of the next page (`None` = last page).
#[derive(Clone, Debug)]
pub(crate) struct RowsPage {
    pub rows: Vec<Row>,
    pub total: usize,
    pub next: Option<String>,
}

/// The window a cursor and a limit mean: (skip, limit). Limit 0 = the default size 100; a larger limit is capped at 1000.
pub(crate) fn window(cursor: &[u8], limit: u32) -> Result<(usize, usize), Refusal> {
    let skip = match cursor {
        [] => 0,
        b => std::str::from_utf8(b)
            .ok()
            .and_then(|t| t.parse::<usize>().ok())
            .ok_or_else(|| refuse("bad-arguments", 400, "bad cursor"))?,
    };
    let limit = match limit {
        0 => 100,
        n => (n as usize).min(1000),
    };
    Ok((skip, limit))
}

/// Whether `form` is the one form the fake serves (any spelling of the key that parses to `cards-in-columns@1.0`).
pub(crate) fn knows_form(form: &str) -> bool {
    parse_key(form).is_some_and(|(n, v)| n == "cards-in-columns" && (v.major, v.minor) == (1, 0))
}

impl FakeSource {
    /// The rows of one page, decided without any RPC (the `page` of the server and of `FakeCore`).
    pub(crate) fn rows(
        &self,
        form: &str,
        scope: &str,
        cursor: &[u8],
        limit: u32,
    ) -> Result<RowsPage, Refusal> {
        if !knows_form(form) {
            return Err(refuse("unknown-form", 404, "no such form"));
        }
        if !scope.is_empty() {
            return Err(refuse("unknown-scope", 404, "the fake has no scopes"));
        }
        let (skip, limit) = window(cursor, limit)?;
        let cards = self.sh.cards.borrow();
        let end = skip.saturating_add(limit).min(cards.len());
        let due = if self.mode == Mode::LaxShape {
            "2026-11-30T12:00:00-04[America/New_York]"
        } else {
            "2026-11-30T12:00:00+01:00"
        };
        let rows = cards
            .get(skip..end)
            .unwrap_or_default()
            .iter()
            .map(|(id, title, col)| {
                let mut values = vec![
                    ("title".to_owned(), title.clone()),
                    ("column".to_owned(), col.clone()),
                ];
                if id == "c1" {
                    values.push(("due".to_owned(), due.to_owned()));
                    values.extend(
                        standard_values(self.mode)
                            .into_iter()
                            .map(|(p, t)| (p.to_owned(), t)),
                    );
                }
                Row {
                    id: id.clone(),
                    values,
                }
            })
            .collect();
        Ok(RowsPage {
            rows,
            total: cards.len(),
            next: (end < cards.len()).then(|| end.to_string()),
        })
    }

    /// May `blob(record, field)` be answered: the fake has an image field only `with_blob`, only card `c1`, field `image`.
    pub(crate) fn check_blob(&self, record: &str, field: &str) -> Result<(), Refusal> {
        if !self.blob {
            return Err(refuse(
                "unsupported",
                501,
                "the fake has no image or file field",
            ));
        }
        if record != "c1" || field != "image" {
            return Err(refuse("unknown-record", 404, "no such image"));
        }
        Ok(())
    }

    /// May the card `id` be asked for its `may` list (a locked card is not changeable).
    pub(crate) fn changeable(id: &str) -> bool {
        id != LOCKED
    }
}
