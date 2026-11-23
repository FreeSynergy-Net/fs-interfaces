//! The runner of the conformance rules.
//!
//! Pattern: Template method (a fixed list of rules, each one checked against the provider under test).

use super::cx::{Cx, Report};
use super::{
    ConformanceFixture, rules_deep, rules_groups, rules_hidden, rules_members, rules_mirror,
    rules_nesting, rules_requests, rules_rights,
};

/// Run every rule against the provider of `fixture` (data is seeded through the trait: the asking person needs
/// `create`). Returns the failed and the skipped rules; empty `failures` means conformant.
pub async fn run_conformance(fixture: &dyn ConformanceFixture) -> Report {
    let Some(mut cx) = Cx::new(fixture).await else {
        return Report {
            failures: vec!["setup: the fixture names no asking person".to_owned()],
            ..Report::default()
        };
    };
    rules_groups::rules(&mut cx).await;
    rules_nesting::rules(&mut cx).await;
    rules_deep::rules(&mut cx).await;
    rules_requests::rules(&mut cx).await;
    rules_members::rules(&mut cx).await;
    rules_rights::rules(&mut cx).await;
    rules_hidden::rules(&mut cx).await;
    rules_mirror::rules(&mut cx).await;
    cx.act_me().await;
    cx.report
}
