//! The ids and the `ChangeRef` of a log entry of the fake (`FakeCore` and the change feed of the level 2 fake).
//!
//! Pattern: Adapter (a log `Entry` of the fake as the value `ChangeRef` of the module contract; the id as UUID text).

use fs_interfaces::module::{ChangeKind, ChangeRef, Origin};
use fs_types::Utc;

use crate::fake_l2::Entry;
use crate::fake_l2_wire::PERSON;

/// The text of a UUID (8-4-4-4-12, lower case): the form a change id has as text.
pub(crate) fn uuid_text(id: u128) -> String {
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
pub(crate) fn uuid_number(text: &str) -> Option<u128> {
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
