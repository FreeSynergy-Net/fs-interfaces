//! Rules of `module_v1`, part 6: the same write twice, a place by neighbours (rules 18, 21). They change data and run after rule 7.
//!
//! Pattern: Template Method (a fixed list of rules, each one checked against the service of the fixture).

use super::cx::{Cx, refused};
use crate::{CommandKind, Verify};

fn borrowed(args: &[(String, String)]) -> Vec<(&str, &str)> {
    args.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect()
}

/// 18. twice: a delete again is unknown-record 404; a create again makes another record (level 1 has no step key; `StepRef` comes later).
pub(crate) async fn twice(cx: &mut Cx<'_>) {
    let kinds = cx.fx.commands();
    let kind_of = |k: &str| kinds.iter().find(|c| c.key == k).map(|c| c.kind);
    let (mut gone, mut made) = (false, false);
    for s in cx.fx.samples() {
        let args = borrowed(&s.args);
        match (&s.verify, kind_of(&s.command)) {
            (Verify::Gone { .. }, Some(CommandKind::Record)) if !gone => {
                gone = true;
                let r = cx.invoke(&s.command, &s.record, &args).await;
                cx.ensure(
                    r.as_ref().is_ok_and(|a| refused(a, "unknown-record", 404)),
                    &format!("18 {} again is unknown-record 404", s.command),
                );
            }
            (_, Some(CommandKind::Set)) if s.makes_record && !made => {
                made = true;
                let (a, b) = (
                    cx.invoke(&s.command, "", &args).await,
                    cx.invoke(&s.command, "", &args).await,
                );
                let ids = a.and_then(|a| b.map(|b| (a, b)));
                cx.ensure(
                    ids.is_ok_and(|(a, b)| {
                        !a.failed && !b.failed && !a.record.is_empty() && a.record != b.record
                    }),
                    &format!("18 {} twice makes two records with two ids", s.command),
                );
            }
            _ => {}
        }
    }
    if let Some(l) = cx.fx.create_without_lane() {
        let r = cx.invoke(&l.command, "", &borrowed(&l.args)).await;
        let ok = r.as_ref().is_ok_and(|a| {
            if l.accepted {
                !a.failed
            } else {
                refused(a, "bad-arguments", 400)
            }
        });
        cx.ensure(
            ok,
            &format!(
                "18 {} without lane: one board = done, none or several = bad-arguments 400",
                l.command
            ),
        );
    }
    if !gone || !made {
        cx.skip("18 twice (the samples have no delete and no create)");
    }
}

/// 21. a place is named by neighbours (`after`, `before`), never by an integer: the record ends up right behind / before the neighbour; `index`
/// and `order` are refused as unknown arguments.
pub(crate) async fn by_neighbours(cx: &mut Cx<'_>) {
    let Some(o) = cx.fx.ordered() else {
        return cx.skip("21 place by neighbours (the fixture has no ordered move)");
    };
    let scope = cx
        .fx
        .forms()
        .into_iter()
        .find(|f| f.key == o.form)
        .map(|f| f.scope)
        .unwrap_or_default();
    let declared = cx.fx.commands().into_iter().find(|c| c.key == o.command);
    cx.ensure(
        declared.is_some_and(|c| {
            ["after", "before"]
                .iter()
                .all(|n| c.params.iter().any(|p| &p.name == n))
                && c.params
                    .iter()
                    .all(|p| p.name != "index" && p.name != "order")
        }),
        &format!(
            "21 {} declares after and before, and no index or order",
            o.command
        ),
    );
    for (name, offset) in [("after", 1isize), ("before", -1)] {
        let mut args = borrowed(&o.args);
        args.push((name, &o.neighbour));
        let r = cx.invoke(&o.command, &o.record, &args).await;
        cx.ensure(
            r.as_ref().is_ok_and(|a| !a.failed),
            &format!("21 {} with {name} is done", o.command),
        );
        let rows = cx.walk(&o.form, &scope, 100).await.unwrap_or_default();
        let at = |id: &str| {
            rows.iter()
                .position(|r| r.id == id)
                .and_then(|i| isize::try_from(i).ok())
        };
        cx.ensure(
            at(&o.record)
                .zip(at(&o.neighbour))
                .is_some_and(|(r, n)| r == n + offset),
            &format!("21 the record is right {name} its neighbour"),
        );
    }
    for integer in ["index", "order"] {
        let mut args = borrowed(&o.args);
        args.push((integer, "0"));
        let r = cx.invoke(&o.command, &o.record, &args).await;
        cx.ensure(
            r.as_ref().is_ok_and(|a| refused(a, "bad-arguments", 400)),
            &format!("21 an integer place (`{integer}`) is bad-arguments 400"),
        );
    }
}
