//! Rules of `module_v1`, part 3: rights, blob, writes (rules 11, 12, 7).
//!
//! Pattern: Template Method (a fixed list of rules, each one checked against the service of the fixture).

use std::collections::HashSet;

use super::cx::{Cx, NOWHERE, first_record, names, refused};
use crate::{CommandKind, Verify};

/// 11. `may` lists only declared commands of the right kind; what is not in `may` is forbidden.
pub(crate) async fn may_and_forbidden(cx: &mut Cx<'_>) {
    let cmds = cx.fx.commands();
    let of = |k: CommandKind| {
        cmds.iter()
            .filter(|c| c.kind == k)
            .map(|c| c.key.clone())
            .collect::<HashSet<_>>()
    };
    let (on_record, on_set) = (of(CommandKind::Record), of(CommandKind::Set));
    for f in cx.fx.forms() {
        let Ok(p) = cx.page(&f.key, &f.scope, &[], 100).await else {
            continue;
        };
        cx.ensure(
            p.set_may.iter().all(|m| on_set.contains(m)),
            "11 RecordPage.may lists only set commands",
        );
        cx.ensure(
            p.rows
                .iter()
                .all(|r| r.may.iter().all(|m| on_record.contains(m))),
            "11 RecordView.may lists only record commands",
        );
    }
    let Some((cmd, record)) = cx.fx.forbidden() else {
        return cx.skip("11 forbidden (the fixture cannot lock a record)");
    };
    let args = cx
        .fx
        .samples()
        .into_iter()
        .find(|s| s.command == cmd)
        .map(|s| s.args)
        .unwrap_or_default();
    let args: Vec<(&str, &str)> = args.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let shown = match cx.fx.forms().into_iter().next() {
        Some(f) => cx
            .walk(&f.key, &f.scope, 100)
            .await
            .ok()
            .and_then(|rows| rows.into_iter().find(|r| r.id == record)),
        None => None,
    };
    cx.ensure(
        shown.is_some_and(|r| !r.may.contains(&cmd)),
        "11 a command not in `may` of the record",
    );
    let r = cx.invoke(&cmd, &record, &args).await;
    cx.ensure(
        r.as_ref().is_ok_and(|a| refused(a, "forbidden", 403)),
        "11 what is not in `may` is forbidden 403 in the answer",
    );
}

/// 12. blob without an image field.
pub(crate) async fn blob(cx: &mut Cx<'_>) {
    if cx.fx.has_blob() {
        return cx.skip("12 blob (the module has an image field; not covered)");
    }
    let rec = first_record(cx).await.unwrap_or_default();
    let mut req = cx.src.blob_request();
    req.get().set_record(&rec);
    req.get().set_field(NOWHERE);
    let r = req.send().promise.await.map(|_| ());
    cx.ensure(
        names(&r, "unsupported"),
        "12 blob without an image field is an exception form-source-unsupported",
    );
}

/// 7. valid invoke: done, problem empty, result set, the effect is read back. Last: it changes data.
pub(crate) async fn invoke_valid(cx: &mut Cx<'_>) {
    let kinds = cx.fx.commands();
    for s in cx.fx.samples() {
        let rule = |t: &str| format!("7 {}: {t}", s.command);
        cx.ensure(
            kinds.iter().any(|c| c.key == s.command),
            &rule("the sample is a declared command"),
        );
        let args: Vec<(&str, &str)> = s
            .args
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let a = match cx.invoke(&s.command, &s.record, &args).await {
            Ok(a) => a,
            Err(e) => {
                cx.ensure(false, &rule(&format!("the call failed: {e}")));
                continue;
            }
        };
        cx.ensure(
            !a.failed && a.kind.is_empty(),
            &rule("failed = false and problem empty"),
        );
        cx.ensure(
            s.makes_record != a.record.is_empty(),
            &rule("result.record is set unless the command made none"),
        );
        match &s.verify {
            Verify::None => {}
            Verify::Value {
                form,
                record,
                path,
                text,
            } => {
                let id = record.clone().unwrap_or_else(|| a.record.clone());
                let scope = cx
                    .fx
                    .forms()
                    .into_iter()
                    .find(|f| &f.key == form)
                    .map(|f| f.scope)
                    .unwrap_or_default();
                let rows = cx.walk(form, &scope, 100).await.unwrap_or_default();
                let seen = rows
                    .iter()
                    .any(|r| r.id == id && r.values.iter().any(|(p, t)| p == path && t == text));
                cx.ensure(seen, &rule("a new read shows the effect"));
            }
            Verify::Gone { form, record } => {
                let scope = cx
                    .fx
                    .forms()
                    .into_iter()
                    .find(|f| &f.key == form)
                    .map(|f| f.scope)
                    .unwrap_or_default();
                let rows = cx.walk(form, &scope, 100).await.unwrap_or_default();
                cx.ensure(
                    rows.iter().all(|r| &r.id != record),
                    &rule("a new read no longer shows the record"),
                );
            }
        }
    }
}
