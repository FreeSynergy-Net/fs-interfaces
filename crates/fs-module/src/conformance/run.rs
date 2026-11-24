//! `run_conformance`: all rules in one order, writes last.
//!
//! Pattern: Template Method (the order of the rules is fixed here; each rule lives in `rules_*`).

use super::cx::{Cx, Report};
use super::{
    rules_blob, rules_errors, rules_level2, rules_limits, rules_order, rules_read, rules_rights,
    rules_shape, rules_values,
};
use crate::FormSourceFixture;

/// Run all rules in `LocalSet` context (capnp clients are not `Send`). Writes happen last (rule 7).
pub async fn run_conformance(fx: &dyn FormSourceFixture) -> Report {
    let mut cx = Cx::new(fx);
    rules_read::hello(&mut cx).await;
    rules_read::forms_listed(&mut cx).await;
    rules_read::reading(&mut cx).await;
    rules_read::keys(&mut cx).await;
    rules_errors::unknown_form(&mut cx).await;
    rules_errors::unknown_scope(&mut cx).await;
    rules_errors::unknown_command(&mut cx).await;
    rules_errors::bad_arguments(&mut cx).await;
    rules_errors::unknown_record(&mut cx).await;
    rules_rights::may_and_forbidden(&mut cx).await;
    rules_rights::forbidden_set(&mut cx).await;
    rules_rights::blob(&mut cx).await;
    rules_blob::blob_bytes(&mut cx).await;
    rules_limits::limits(&mut cx).await;
    rules_limits::keys_strict(&mut cx).await;
    rules_shape::host_id(&mut cx).await;
    rules_shape::hidden_fields(&mut cx).await;
    rules_level2::level2(&mut cx).await;
    rules_rights::invoke_valid(&mut cx).await;
    rules_order::twice(&mut cx).await;
    rules_order::by_neighbours(&mut cx).await;
    rules_values::standard_values(&mut cx).await;
    rules_shape::refusal_shape(&mut cx);
    cx.report
}
