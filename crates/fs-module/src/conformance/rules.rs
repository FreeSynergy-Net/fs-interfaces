//! The rules 1-12 (13 is the self test).

use std::collections::HashSet;

use fs_interfaces::forms::{FORM_SOURCE_LEVEL, find_key, parse_key};

use super::cx::{Cx, names};
use crate::{CommandKind, Verify};

const NOWHERE: &str = "zz-no-such";

fn short(key: &str) -> Option<String> {
    let (n, v) = parse_key(key)?;
    Some(format!("{n}@{}.{}", v.major, v.minor))
}

/// 1. hello: level, provider.
pub(crate) async fn hello(cx: &mut Cx<'_>) {
    let r = async { cx.src.hello_request().send().promise.await }.await;
    let Ok(r) = r else {
        return cx.ensure(false, "1 hello: the call failed");
    };
    let Ok(a) = r.get() else {
        return cx.ensure(false, "1 hello: the answer is unreadable");
    };
    cx.ensure(
        a.get_level() == FORM_SOURCE_LEVEL,
        "1 hello: level is FORM_SOURCE_LEVEL",
    );
    let provider = a
        .get_provider()
        .ok()
        .and_then(|p| p.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    cx.ensure(!provider.is_empty(), "1 hello: provider is not empty");
    if let Some(want) = cx.fx.provider() {
        cx.ensure(provider == want, "1 hello: provider is the declared one");
    }
}

/// 2. every declared form is in the catalog and `page` works with it.
pub(crate) async fn forms_listed(cx: &mut Cx<'_>) {
    for f in cx.fx.forms() {
        cx.ensure(
            find_key(&f.key).is_some(),
            &format!("2 forms: {} is in the catalog", f.key),
        );
        let ok = cx.page(&f.key, &f.scope, &[], 1).await.is_ok();
        cx.ensure(ok, &format!("2 forms: page works for {}", f.key));
    }
}

/// 3. reading: ids, paths, limit, cursor, total.
pub(crate) async fn reading(cx: &mut Cx<'_>) {
    for f in cx.fx.forms() {
        let rule = |t: &str| format!("3 reading {}: {t}", f.key);
        let all = match cx.page(&f.key, &f.scope, &[], 100).await {
            Ok(p) => p,
            Err(e) => {
                cx.ensure(false, &rule(&format!("page failed: {e}")));
                continue;
            }
        };
        let ids: Vec<&str> = all.rows.iter().map(|r| r.id.as_str()).collect();
        cx.ensure(
            ids.iter().all(|i| !i.is_empty())
                && ids.iter().collect::<HashSet<_>>().len() == ids.len(),
            &rule("ids are not empty and unique"),
        );
        cx.ensure(
            all.rows
                .iter()
                .all(|r| r.values.iter().all(|(p, _)| f.fields.contains(p))),
            &rule("value paths are declared fields"),
        );
        cx.ensure(all.rows.len() <= 100, &rule("records <= limit"));
        let one = cx.page(&f.key, &f.scope, &[], 1).await;
        cx.ensure(
            one.as_ref().is_ok_and(|p| p.rows.len() <= 1),
            &rule("records <= limit (1)"),
        );
        match cx.walk(&f.key, &f.scope, 1).await {
            Ok(walked) => {
                let w: Vec<&str> = walked.iter().map(|r| r.id.as_str()).collect();
                cx.ensure(w == ids, &rule("limit 1 with next shows every record exactly once, the end is an empty cursor"));
            }
            Err(e) => cx.ensure(false, &rule(&format!("walk failed: {e}"))),
        }
        if all.has_total {
            cx.ensure(
                all.total as usize == ids.len(),
                &rule("hasTotal => total = count"),
            );
        }
    }
}

/// 4. `x.y.z` and `x.y` both work; another major is unknown.
pub(crate) async fn keys(cx: &mut Cx<'_>) {
    for f in cx.fx.forms() {
        let (Some(s), Some((n, v))) = (short(&f.key), parse_key(&f.key)) else {
            cx.ensure(false, &format!("4 keys: {} is not a key", f.key));
            continue;
        };
        let (a, b) = (
            cx.page(&f.key, &f.scope, &[], 100).await,
            cx.page(&s, &f.scope, &[], 100).await,
        );
        let same = match (&a, &b) {
            (Ok(a), Ok(b)) => a
                .rows
                .iter()
                .map(|r| &r.id)
                .eq(b.rows.iter().map(|r| &r.id)),
            _ => false,
        };
        cx.ensure(same, &format!("4 keys: {} and {s} answer the same", f.key));
        let other = cx
            .page(&format!("{n}@{}.0.0", v.major + 1), &f.scope, &[], 1)
            .await;
        cx.ensure(
            names(&other, "unknown-form"),
            &format!("4 keys: another major of {n} is unknown-form"),
        );
    }
}

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

fn refused(a: &super::cx::Answer, kind: &str, status: u16) -> bool {
    a.failed && a.kind == format!("form-source-{kind}") && a.status == status
}

/// 8. unknown command: in the answer, no exception.
pub(crate) async fn unknown_command(cx: &mut Cx<'_>) {
    let r = cx.invoke(&format!("{NOWHERE}.command"), "", &[]).await;
    cx.ensure(
        r.as_ref().is_ok_and(|a| refused(a, "unknown-command", 404)),
        "8 unknown command is failed + unknown-command 404 in the answer, no exception",
    );
}

async fn first_record(cx: &Cx<'_>) -> Option<String> {
    let f = cx.fx.forms().into_iter().next()?;
    cx.page(&f.key, &f.scope, &[], 1)
        .await
        .ok()?
        .rows
        .into_iter()
        .next()
        .map(|r| r.id)
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
            s.makes_record == !a.record.is_empty(),
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
