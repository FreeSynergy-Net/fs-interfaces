//! Rule 22 against the fake: the eight standard channels and the counter-proofs (a wrong value, a repeated value).

mod common;

use common::run;
use fs_module::{FakeSource, Mode};
use tokio::task::LocalSet;

#[tokio::test(flavor = "current_thread")]
async fn lax_values_must_turn_rule_22_red_for_each_of_the_eight_channels() {
    let r = LocalSet::new()
        .run_until(run(Mode::LaxValues, true, 1))
        .await;
    for path in [
        "description",
        "color",
        "estimated",
        "column-limit",
        "priority",
        "state",
        "time-zone",
        "assignee",
    ] {
        assert!(
            r.failures
                .iter()
                .any(|f| f.starts_with("22 ") && f.contains(&format!("22 {path} "))),
            "{path}: {:#?}",
            r.failures
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_repeated_value_at_another_channel_than_assignee_must_turn_rule_22_red() {
    let r = LocalSet::new()
        .run_until(run(Mode::LaxRepeat, true, 1))
        .await;
    assert!(
        r.failures
            .iter()
            .any(|f| f.starts_with("22 priority ") && f.contains("at most one value")),
        "{:#?}",
        r.failures
    );
    // Only the repeat is wrong: the assignee (two values) and every value itself stay accepted.
    assert!(
        r.failures.iter().all(|f| f.contains("22 priority ")),
        "{:#?}",
        r.failures
    );
}

#[tokio::test(flavor = "current_thread")]
async fn the_strict_fake_serves_two_assignees_and_rule_22_accepts_them() {
    let r = LocalSet::new().run_until(run(Mode::Strict, true, 1)).await;
    assert!(r.failures.is_empty(), "{:#?}", r.failures);
    let rows = LocalSet::new()
        .run_until(async {
            let c = FakeSource::client(Mode::Strict);
            let mut req = c.page_request();
            {
                let mut p = req.get();
                p.set_form("cards-in-columns@1.0.0");
                p.set_limit(100);
                p.init_cursor().set_opaque(b"");
            }
            let res = req.send().promise.await.unwrap();
            let page = res.get().unwrap().get_page().unwrap();
            let mut got = Vec::new();
            for rec in page.get_records().unwrap() {
                for v in rec.get_values().unwrap() {
                    got.push((
                        v.get_path().unwrap().to_string().unwrap(),
                        v.get_text().unwrap().to_string().unwrap(),
                    ));
                }
            }
            got
        })
        .await;
    let n = |p: &str| rows.iter().filter(|(k, _)| k == p).count();
    assert_eq!(n("assignee"), 2);
    assert_eq!((n("state"), n("time-zone"), n("priority")), (1, 1, 1));
}
