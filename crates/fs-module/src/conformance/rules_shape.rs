//! Rules of `module_v1`, part 5: the shape of a refusal, host ids, `may` against `forbidden` (rules 14, 17, 19).
//!
//! Pattern: Template Method (a fixed list of rules, each one checked against the service of the fixture).

use super::cx::{Cx, Seen, has_word};

/// The problem kinds and their status (`formsource.capnp`); `unknown-scope` also has 409.
const KINDS: [(&str, &[u16]); 10] = [
    ("unknown-form", &[404]),
    ("unknown-scope", &[404, 409]),
    ("bad-arguments", &[400]),
    ("unsupported", &[501]),
    ("unknown-record", &[404]),
    ("unknown-command", &[404]),
    ("forbidden", &[403]),
    ("rejected", &[422]),
    ("conflict", &[409]),
    ("internal", &[500]),
];

/// Rule 14: every refusal the suite saw has the right shape: an exception names exactly one kind of the table (never a foreign problem URI);
/// an `invoke` answer has a type of the table (a whole word `form-source-<kind>`), the status of that kind, a title that is a text for
/// people (not the kind), and an empty `result` (a refused write names no record). Texts are UTF-8 (reading them fails otherwise).
pub(crate) fn refusal_shape(cx: &mut Cx<'_>) {
    let seen = std::mem::take(&mut *cx.seen.borrow_mut());
    cx.ensure(!seen.is_empty(), "14 the suite saw at least one refusal");
    for s in &seen {
        match s {
            Seen::Exception(text) => {
                let known = KINDS
                    .iter()
                    .any(|(k, _)| has_word(text, &format!("form-source-{k}")));
                cx.ensure(
                    known && !text.contains("freesynergy.net/problems"),
                    &format!("14 an exception names a kind of the table, no foreign URI: {text}"),
                );
            }
            Seen::Answer(a) => {
                let kind = a.kind.strip_prefix("form-source-").unwrap_or("");
                let status_ok = KINDS
                    .iter()
                    .any(|(k, st)| *k == kind && st.contains(&a.status));
                cx.ensure(
                    status_ok,
                    &format!(
                        "14 refusal `{}` has the status of its kind ({})",
                        a.kind, a.status
                    ),
                );
                cx.ensure(
                    !a.title.is_empty() && !a.title.starts_with("form-source-") && a.title != kind,
                    &format!(
                        "14 refusal `{}` has a title that is text for people",
                        a.kind
                    ),
                );
                cx.ensure(
                    a.record.is_empty() && a.text.is_empty(),
                    &format!("14 refusal `{}` leaves `result` empty", a.kind),
                );
            }
        }
    }
}

/// Rule 17: a host id (`planning.card.move`) is not a command of the contract: unknown-command 404 in the answer.
pub(crate) async fn host_id(cx: &mut Cx<'_>) {
    let Some(c) = cx.fx.commands().into_iter().next() else {
        return cx.skip("17 host id (the module declares no command)");
    };
    let r = cx.invoke(&format!("host.{}", c.key), "", &[]).await;
    cx.ensure(
        r.as_ref()
            .is_ok_and(|a| super::cx::refused(a, "unknown-command", 404)),
        "17 a host id is unknown-command 404, not the contract key",
    );
}

/// Rule 19: a hidden field is a declared field and carries no value in the same record (that `may` is not `forbidden` is rule 9).
pub(crate) async fn hidden_fields(cx: &mut Cx<'_>) {
    for f in cx.fx.forms() {
        let Ok(p) = cx.page(&f.key, &f.scope, &[], 100).await else {
            continue;
        };
        for r in &p.rows {
            cx.ensure(
                r.hidden
                    .iter()
                    .all(|h| f.fields.contains(h) && r.values.iter().all(|(v, _)| v != h)),
                &format!("19 {}: a hidden field is declared and has no value", r.id),
            );
        }
    }
}
