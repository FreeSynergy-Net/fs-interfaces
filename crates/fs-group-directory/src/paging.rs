//! Paging over a sorted list with a text cursor (the key of the last item given).
//!
//! Pattern: Iterator-like cursor (pure function over a keyed list).

use crate::groups_v1::shared::{Page, Place};

/// The items after the cursor, at most `size` (at least one), in key order; `next` when more follow.
pub(crate) fn page<T>(mut items: Vec<(String, T)>, after: Option<&Place>, size: usize) -> Page<T> {
    items.sort_by(|a, b| a.0.cmp(&b.0));
    let mut rest: Vec<(String, T)> = match after {
        Some(Place(cursor)) => items.into_iter().filter(|(k, _)| k > cursor).collect(),
        None => items,
    };
    let more = rest.len() > size.max(1);
    rest.truncate(size.max(1));
    let next = if more {
        rest.last().map(|(k, _)| Place(k.clone()))
    } else {
        None
    };
    Page {
        items: rest.into_iter().map(|(_, v)| v).collect(),
        next,
    }
}
