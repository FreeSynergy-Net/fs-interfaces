//! The 13 rules of `module_v1` (FormSource level 1), runnable against any [`FormSourceFixture`](crate::FormSourceFixture).
//!
//! Pattern: Template Method (a fixed list of rules; the fixture supplies client, declared forms and valid calls). A rule the fixture
//! cannot set up is reported as skipped, never as passed. Rule 13 (the counter-proof: a fake that breaks a rule must turn red) is
//! the self test in `tests/selftest.rs`.
//!
//! Deviation (honest): no `fs-rpc` here, so an RPC exception is checked by "exception text contains `form-source-<kind>`".

mod cx;
mod rules_errors;
mod rules_read;
mod rules_rights;

pub use cx::Report;

use crate::FormSourceFixture;

/// Run all rules in `LocalSet` context (capnp clients are not `Send`). Writes happen last (rule 7).
pub async fn run_conformance(fx: &dyn FormSourceFixture) -> Report {
    let mut cx = cx::Cx::new(fx);
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
    rules_rights::blob(&mut cx).await;
    rules_rights::invoke_valid(&mut cx).await;
    cx.report
}
