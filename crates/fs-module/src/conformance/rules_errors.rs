//! Rules of `module_v1`, part 2: errors (rules 5, 6, 8, 9, 10).
//!
//! Pattern: Template Method (a fixed list of rules, each one checked against the service of the fixture).

use fs_interfaces::forms::FORM_SOURCE_LEVEL;

use super::cx::{Cx, NOWHERE, refused};
use super::cx_level2 as level2;
use crate::CommandKind;

/// Rule 5: unknown form = RPC exception `form-source-unknown-form`.
pub(crate) async fn unknown_form(cx: &mut Cx<'_>) {
    let r = cx.page(&format!("{NOWHERE}@1.0.0"), "", &[], 1).await;
    cx.ensure(
        cx.names(&r, "unknown-form"),
        "5 unknown form is an exception form-source-unknown-form",
    );
}

/// Rule 6: unknown scope.
pub(crate) async fn unknown_scope(cx: &mut Cx<'_>) {
    let (true, Some(f)) = (cx.fx.strict_scope(), cx.fx.forms().into_iter().next()) else {
        return cx.skip("6 unknown scope (the fixture names no strict scope)");
    };
    let r = cx.page(&f.key, NOWHERE, &[], 1).await;
    cx.ensure(
        cx.names(&r, "unknown-scope"),
        "6 unknown scope is an exception form-source-unknown-scope",
    );
    // `watch` and `changes` exist from level 2 on; there the same refusal is due, before anything is subscribed or read.
    if cx.fx.level().min(FORM_SOURCE_LEVEL) < 2 {
        return cx.skip("6 unknown scope at watch and changes (level 1 has neither)");
    }
    let r = level2::watch_in(&cx.src, &f.key, NOWHERE).await;
    cx.ensure(
        cx.names(&r, "unknown-scope"),
        "6 unknown scope at watch is an exception form-source-unknown-scope",
    );
    let r = level2::changes_in(&cx.src, NOWHERE).await;
    cx.ensure(
        cx.names(&r, "unknown-scope"),
        "6 unknown scope at changes is an exception form-source-unknown-scope",
    );
}

/// Rule 8: unknown command: in the answer, no exception.
pub(crate) async fn unknown_command(cx: &mut Cx<'_>) {
    let r = cx.invoke(&format!("{NOWHERE}.command"), "", &[]).await;
    cx.ensure(
        r.as_ref().is_ok_and(|a| refused(a, "unknown-command", 404)),
        "8 unknown command is failed + unknown-command 404 in the answer, no exception",
    );
}

/// Rule 9: unknown, doubled or missing arguments. Asked on a record (or the set) the person MAY use the command on: the rights come before
/// the arguments (see rule 11), so a locked record would answer `forbidden` and say nothing about the arguments.
pub(crate) async fn bad_arguments(cx: &mut Cx<'_>) {
    let Some(f) = cx.fx.forms().into_iter().next() else {
        return cx.skip("9 bad arguments (no form to read)");
    };
    let Ok(page) = cx.page(&f.key, &f.scope, &[], 100).await else {
        return cx.ensure(false, "9 bad arguments: the page failed");
    };
    for c in cx.fx.commands() {
        let record = match c.kind {
            CommandKind::Record => page
                .rows
                .iter()
                .find(|r| r.may.contains(&c.key))
                .map(|r| r.id.clone()),
            CommandKind::Set => page.set_may.contains(&c.key).then(String::new),
        };
        let Some(record) = record else {
            cx.skip(&format!(
                "9 {}: no record or set where the person may use it",
                c.key
            ));
            continue;
        };
        let record = record.as_str();
        let r = cx.invoke(&c.key, record, &[("zz-unknown", "x")]).await;
        cx.ensure(
            r.as_ref().is_ok_and(|a| refused(a, "bad-arguments", 400)),
            &format!("9 {}: unknown argument is bad-arguments 400", c.key),
        );
        if let Some(p) = c.params.first() {
            let r = cx
                .invoke(
                    &c.key,
                    record,
                    &[(p.name.as_str(), "x"), (p.name.as_str(), "x")],
                )
                .await;
            cx.ensure(
                r.as_ref().is_ok_and(|a| refused(a, "bad-arguments", 400)),
                &format!("9 {}: doubled argument is bad-arguments 400", c.key),
            );
        }
        if c.params.iter().any(|p| p.required) {
            let r = cx.invoke(&c.key, record, &[]).await;
            cx.ensure(
                r.as_ref().is_ok_and(|a| refused(a, "bad-arguments", 400)),
                &format!(
                    "9 {}: a missing required argument is bad-arguments 400",
                    c.key
                ),
            );
        }
    }
}

/// Rule 10: unknown record (with valid arguments).
pub(crate) async fn unknown_record(cx: &mut Cx<'_>) {
    let kinds = cx.fx.commands();
    let Some(s) = cx.fx.samples().into_iter().find(|s| {
        kinds
            .iter()
            .any(|c| c.key == s.command && c.kind == CommandKind::Record)
    }) else {
        return cx.skip("10 unknown record (no sample of a record command)");
    };
    let args: Vec<(&str, &str)> = s
        .args
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let r = cx.invoke(&s.command, NOWHERE, &args).await;
    cx.ensure(
        r.as_ref().is_ok_and(|a| refused(a, "unknown-record", 404)),
        "10 unknown record is unknown-record 404 in the answer",
    );
}
