//! Rules of `module_v1`, part 2: errors (rules 5, 6, 8, 9, 10).

use super::cx::{Cx, NOWHERE, first_record, names, refused};
use crate::CommandKind;

/// 5. unknown form = RPC exception `form-source-unknown-form`.
pub(crate) async fn unknown_form(cx: &mut Cx<'_>) {
    let r = cx.page(&format!("{NOWHERE}@1.0.0"), "", &[], 1).await;
    cx.ensure(
        names(&r, "unknown-form"),
        "5 unknown form is an exception form-source-unknown-form",
    );
}

/// 6. unknown scope.
pub(crate) async fn unknown_scope(cx: &mut Cx<'_>) {
    let (true, Some(f)) = (cx.fx.strict_scope(), cx.fx.forms().into_iter().next()) else {
        return cx.skip("6 unknown scope (the fixture names no strict scope)");
    };
    let r = cx.page(&f.key, NOWHERE, &[], 1).await;
    cx.ensure(
        names(&r, "unknown-scope"),
        "6 unknown scope is an exception form-source-unknown-scope",
    );
}

/// 8. unknown command: in the answer, no exception.
pub(crate) async fn unknown_command(cx: &mut Cx<'_>) {
    let r = cx.invoke(&format!("{NOWHERE}.command"), "", &[]).await;
    cx.ensure(
        r.as_ref().is_ok_and(|a| refused(a, "unknown-command", 404)),
        "8 unknown command is failed + unknown-command 404 in the answer, no exception",
    );
}

/// 9. unknown, doubled or missing arguments.
pub(crate) async fn bad_arguments(cx: &mut Cx<'_>) {
    let Some(rec) = first_record(cx).await else {
        return cx.skip("9 bad arguments (no record to ask with)");
    };
    for c in cx.fx.commands() {
        let record = if c.kind == CommandKind::Record {
            rec.as_str()
        } else {
            ""
        };
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

/// 10. unknown record (with valid arguments).
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
