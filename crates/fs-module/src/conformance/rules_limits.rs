//! Rules of `module_v1`, part 4: limits and keys (rules 15, 16).
//!
//! Pattern: Template Method (a fixed list of rules, each one checked against the service of the fixture).

use fs_interfaces::forms::parse_key;

use super::cx::Cx;

/// Rule 15: paging at the edges: `limit` 0 (= the service's default), the largest `limit`, an empty page has an empty `next`, a garbage cursor.
pub(crate) async fn limits(cx: &mut Cx<'_>) {
    for f in cx.fx.forms() {
        let rule = |t: &str| format!("15 limits {}: {t}", f.key);
        let Ok(all) = cx.walk(&f.key, &f.scope, 100).await else {
            cx.ensure(false, &rule("the walk failed"));
            continue;
        };
        let ids: Vec<&str> = all.iter().map(|r| r.id.as_str()).collect();
        for limit in [0, u32::MAX] {
            match cx.page(&f.key, &f.scope, &[], limit).await {
                Ok(p) => {
                    let got: Vec<&str> = p.rows.iter().map(|r| r.id.as_str()).collect();
                    cx.ensure(
                        ids.is_empty() || !got.is_empty(),
                        &rule(&format!("limit {limit} gives a page that is not empty")),
                    );
                    cx.ensure(
                        ids.starts_with(&got),
                        &rule(&format!("limit {limit} gives the first records, in order")),
                    );
                    cx.ensure(
                        !got.is_empty() || p.next.is_empty(),
                        &rule(&format!("limit {limit}: an empty page has an empty next")),
                    );
                }
                Err(e) => cx.ensure(false, &rule(&format!("limit {limit} failed: {e}"))),
            }
        }
        let walked = cx.walk(&f.key, &f.scope, 0).await;
        cx.ensure(
            walked.is_ok_and(|w| w.iter().map(|r| r.id.as_str()).eq(ids.iter().copied())),
            &rule("limit 0 with next shows every record exactly once"),
        );
        for junk in [vec![0xff, 0xfe, 0xfd], vec![0xa5; 1 << 20]] {
            let r = cx.page(&f.key, &f.scope, &junk, 1).await;
            cx.ensure(
                cx.names(&r, "bad-arguments"),
                &rule(&format!(
                    "a garbage cursor ({} bytes) is bad-arguments",
                    junk.len()
                )),
            );
        }
    }
}

/// Rule 16: keys are strict: the patch does not count, a higher minor and every broken spelling is unknown-form (an exception, no panic).
pub(crate) async fn keys_strict(cx: &mut Cx<'_>) {
    for f in cx.fx.forms() {
        let Some((n, v)) = parse_key(&f.key) else {
            continue;
        };
        let rule = |t: &str| format!("16 keys {}: {t}", f.key);
        let exact = cx.page(&f.key, &f.scope, &[], 100).await;
        let patched = cx
            .page(
                &format!("{n}@{}.{}.7", v.major, v.minor),
                &f.scope,
                &[],
                100,
            )
            .await;
        let same = match (&exact, &patched) {
            (Ok(a), Ok(b)) => a
                .rows
                .iter()
                .map(|r| &r.id)
                .eq(b.rows.iter().map(|r| &r.id)),
            _ => false,
        };
        cx.ensure(same, &rule("another patch number answers the same"));
        let (major, minor) = (v.major, v.minor);
        let broken = [
            format!("{n}@{major}.{}.0", minor + 1),
            format!("{n}@{major}"),
            format!("{n}@{major}.{minor}.0.0"),
            format!("{n}@0{major}.{minor}.0"),
            format!("{n}@v{major}.{minor}.0"),
            n.to_owned(),
            format!("{n}@@{major}.{minor}.0"),
            format!("{n}@{major}.{minor}.0@{major}.{minor}.0"),
        ];
        for key in &broken {
            let r = cx.page(key, &f.scope, &[], 1).await;
            cx.ensure(
                cx.names(&r, "unknown-form"),
                &rule(&format!("`{key}` is unknown-form")),
            );
        }
    }
}
