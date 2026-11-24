//! Rules of `module_v1`, part 1: reading (rules 1-4).

use std::collections::HashSet;

use fs_interfaces::forms::{FORM_SOURCE_LEVEL, find_key, parse_key};

use super::cx::{Cx, names};

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
                usize::try_from(all.total).is_ok_and(|t| t == ids.len()),
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
