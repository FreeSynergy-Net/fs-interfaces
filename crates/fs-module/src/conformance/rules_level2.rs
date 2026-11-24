//! Rules L2-2 to L2-11 of `module_v1` level 2 (Stufe 3): compare-and-set, one effect per step, undo, other programs, push.
//! L2-1 is rule 1 (`hello`); L2-12 (the counter-proof) is the self test.
//!
//! Pattern: Template Method (a fixed list of rules; the fixture supplies client, a second connection and a valid sample).
//! A fixture of level 1 skips them all, never passes them. They change data and put it back (undo) where they can.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use fs_interfaces::forms::FORM_SOURCE_LEVEL;
use fs_interfaces::formsource_capnp::change_sink;

use super::cx::Cx;
use super::cx_level2::{self as cx, Change, IfAnswer, is_v7};
use crate::{CommandKind, Sample};

const RUN: &str = "conf-run-a";
const RUN_UNDO: &str = "conf-run-undo";
const PROGRAM: &str = "conformance";
const OTHER: &str = "other";

type State = Vec<(String, Vec<(String, String)>)>;

async fn state(cx: &Cx<'_>, form: &str, scope: &str) -> Option<State> {
    let rows = cx.walk(form, scope, 100).await.ok()?;
    Some(rows.into_iter().map(|r| (r.id, r.values)).collect())
}

fn conflict(a: &IfAnswer) -> bool {
    a.failed && a.kind == "form-source-conflict" && a.status == 409
}

fn is_ok(a: &capnp::Result<IfAnswer>) -> Option<&IfAnswer> {
    a.as_ref().ok().filter(|a| !a.failed)
}

/// The sample every rule changes: a command on one record that makes a record.
fn pick(cx: &Cx<'_>) -> Option<Sample> {
    let kinds = cx.fx.commands();
    cx.fx.samples().into_iter().find(|s| {
        !s.record.is_empty()
            && s.makes_record
            && kinds
                .iter()
                .any(|c| c.key == s.command && c.kind == CommandKind::Record)
    })
}

/// Rules L2-2 to L2-11, in this order (each one builds on the one before).
#[allow(clippy::too_many_lines)] // one block per rule, in the order of the list; splitting would hide the order
pub(crate) async fn level2(cx: &mut Cx<'_>) {
    const RULES: [&str; 10] = [
        "L2-2 invokeIf",
        "L2-3 same step once",
        "L2-4 stale expect",
        "L2-5 revert",
        "L2-6 revert of the undo",
        "L2-7 revert twice, same step",
        "L2-8 another program changed",
        "L2-9 unknown or redacted change",
        "L2-10 changes filter",
        "L2-11 watch sees another program",
    ];
    let (Some(f), Some(s)) = (cx.fx.forms().into_iter().next(), pick(cx)) else {
        return RULES
            .iter()
            .for_each(|r| cx.skip(&format!("{r}: no form or sample that changes one record")));
    };
    // Levels are cumulative: a newer service is checked only as far as this suite knows (`min(level, LEVEL)`).
    if cx.fx.level().min(FORM_SOURCE_LEVEL) < 2 {
        return RULES
            .iter()
            .for_each(|r| cx.skip(&format!("{r}: the service is level 1")));
    }
    let src = cx.src.clone();
    let Ok(()) = cx::introduce(&src, PROGRAM, "inst-1", "dev-1").await else {
        return cx.ensure(false, "L2-2 introduce failed");
    };
    let call = (s.command.as_str(), s.record.as_str(), s.args.as_slice());
    let Some(before) = state(cx, &f.key, &f.scope).await else {
        return cx.ensure(false, "L2-2 page failed");
    };

    // L2-2
    let a = cx::invoke_if(&src, call, "", (RUN, "s-a")).await;
    let change = is_ok(&a).and_then(|a| a.change.clone());
    let sane = change.as_ref().is_some_and(|c| {
        is_v7(c.activity)
            && c.object == s.record
            && c.revision.parse::<u64>().ok() > c.before.parse::<u64>().ok()
    });
    cx.ensure(sane, "L2-2 invokeIf: result.change is set, activity is a UUIDv7, object is the record, revision is above before");
    let Some(change) = change.filter(|_| sane) else {
        return RULES[2..]
            .iter()
            .for_each(|r| cx.skip(&format!("{r}: L2-2 failed")));
    };
    let after = state(cx, &f.key, &f.scope).await;

    // L2-3
    let again = cx::invoke_if(&src, call, "", (RUN, "s-a")).await;
    let in_run = cx::changes(&src, RUN).await;
    cx.ensure(
        is_ok(&again).and_then(|a| a.change.as_ref()) == Some(&change) && in_run.as_ref().is_ok_and(|l| l.len() == 1),
        "L2-3 invokeIf twice with the same step: one change, the same ChangeRef, changes shows one entry",
    );

    // L2-4
    let stale = cx::invoke_if(&src, call, "0", (RUN, "s-stale")).await;
    let clash = stale
        .as_ref()
        .is_ok_and(|a| conflict(a) && a.by.as_ref().is_some_and(|b| b.object == s.record));
    cx.ensure(
        clash && state(cx, &f.key, &f.scope).await == after,
        "L2-4 invokeIf with a stale expect: conflict 409, conflict.by set, the record is unchanged",
    );

    // L2-5
    let undone = cx::revert(&src, change.activity, (RUN_UNDO, "s-r")).await;
    let undo = is_ok(&undone).and_then(|a| a.change.clone());
    let back = state(cx, &f.key, &f.scope).await;
    if before == after.clone().unwrap_or_default() {
        cx.skip("L2-5 revert: the sample changes nothing visible here");
    } else {
        cx.ensure(
            back.as_ref() == Some(&before)
            && undo.as_ref().is_some_and(|u| u.kind == "Undo" && u.undoes == change.activity),
        "L2-5 revert of an unchanged record: the state is as before, the answer is kind Undo with undoes",
        );
    }

    // L2-6
    let redo = match &undo {
        Some(u) => cx::revert(&src, u.activity, (RUN_UNDO, "s-redo"))
            .await
            .ok(),
        None => None,
    };
    cx.ensure(
        redo.as_ref().is_some_and(|a| !a.failed) && state(cx, &f.key, &f.scope).await == after,
        "L2-6 revert of the undo: the state is as after the change (redo)",
    );

    // L2-7
    let redo_change = redo.and_then(|a| a.change);
    let (r1, r2) = match &redo_change {
        Some(c) => (
            cx::revert(&src, c.activity, (RUN_UNDO, "s-twice")).await,
            cx::revert(&src, c.activity, (RUN_UNDO, "s-twice")).await,
        ),
        None => (
            Err(capnp::Error::failed("no change".into())),
            Err(capnp::Error::failed("no change".into())),
        ),
    };
    cx.ensure(
        is_ok(&r1).is_some()
            && is_ok(&r2).is_some()
            && is_ok(&r1).map(|a| &a.change) == is_ok(&r2).map(|a| &a.change),
        "L2-7 revert twice with the same step: one undo, no conflict, the same answer",
    );

    // L2-8, L2-10, L2-11 need a second connection.
    let a2 = cx::invoke_if(&src, call, "", (RUN, "s-a2")).await;
    let second = cx.fx.second();
    match (&second, is_ok(&a2).and_then(|a| a.change.clone())) {
        (Some(two), Some(mine)) => {
            let _ = cx::introduce(two, OTHER, "inst-2", "dev-2").await;
            let theirs = cx::invoke_if(two, call, "", ("conf-run-b", "s-b")).await;
            let mid = state(cx, &f.key, &f.scope).await;
            let late = cx::revert(&src, mine.activity, (RUN_UNDO, "s-late")).await;
            cx.ensure(
                is_ok(&theirs).is_some()
                    && late.as_ref().is_ok_and(|a| {
                        conflict(a) && a.by.as_ref().is_some_and(|b| b.program == OTHER)
                    })
                    && state(cx, &f.key, &f.scope).await == mid,
                "L2-8 revert after another program changed the record: conflict 409, conflict.by is the other program, nothing is overwritten",
            );
        }
        _ => cx.skip("L2-8 another program changed: no second connection"),
    }

    // L2-9
    let unknown = cx::revert(&src, u128::from(u64::MAX) << 70 | 1, (RUN_UNDO, "s-nobody")).await;
    cx.ensure(
        unknown
            .as_ref()
            .is_ok_and(|a| a.failed && a.kind == "form-source-unknown-change" && a.status == 404),
        "L2-9 revert of an unknown change: unknown-change 404",
    );
    if !cx.fx.can_redact() {
        cx.skip("L2-9 gone: the fixture cannot black out a change");
    }

    // L2-10
    let (own, run_a) = (cx::changes(&src, "").await, cx::changes(&src, RUN).await);
    let mut ok = own
        .as_ref()
        .is_ok_and(|l| !l.is_empty() && l.iter().all(|c| c.program == PROGRAM))
        && run_a.as_ref().is_ok_and(|l| {
            l.iter().any(|c| c.activity == change.activity) && l.iter().all(|c| c.kind != "Undo")
        });
    if let Some(two) = &second {
        let theirs = cx::changes(two, "").await;
        ok = ok
            && theirs
                .as_ref()
                .is_ok_and(|l| !l.is_empty() && l.iter().all(|c| c.program == OTHER));
    }
    cx.ensure(ok, "L2-10 changes: run empty = the changes of the own introduce program; run set = only those of that run");

    // L2-11
    let Some(two) = second else {
        return cx.skip("L2-11 watch sees another program: no second connection");
    };
    let heard: Rc<RefCell<Vec<Change>>> = Rc::default();
    let sink: change_sink::Client = capnp_rpc::new_client(cx::Heard(Rc::clone(&heard)));
    let mut w = two.watch_request();
    {
        let mut p = w.get();
        p.set_form(&f.key);
        p.set_scope(&f.scope);
        p.set_sink(sink);
    }
    let sub = w.send().promise.await;
    let pushed = cx::invoke_if(&src, call, "", (RUN, "s-push")).await;
    let mut seen = false;
    for _ in 0..50 {
        seen = heard
            .borrow()
            .iter()
            .any(|c| c.program == PROGRAM && c.object == s.record);
        if seen {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    cx.ensure(
        sub.is_ok() && is_ok(&pushed).is_some() && seen,
        "L2-11 watch on the second connection sees the change of the first within 1 s, with its program",
    );
}
