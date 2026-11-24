//! The suite against the fake in this process, and the counter-proof (rule 13): a fake that breaks a rule must turn the suite red.

use fs_interfaces::formsource_capnp::form_source;
use fs_module::conformance::run_conformance;
use fs_module::{DeclaredCommand, DeclaredForm, FakeSource, FormSourceFixture, Mode, Sample};
use tokio::task::LocalSet;

struct Fixture(form_source::Client);

impl FormSourceFixture for Fixture {
    fn client(&self) -> form_source::Client {
        self.0.clone()
    }
    fn provider(&self) -> Option<&str> {
        Some("fake")
    }
    fn forms(&self) -> Vec<DeclaredForm> {
        FakeSource::forms()
    }
    fn commands(&self) -> Vec<DeclaredCommand> {
        FakeSource::commands()
    }
    fn samples(&self) -> Vec<Sample> {
        FakeSource::samples()
    }
    fn strict_scope(&self) -> bool {
        true
    }
    fn forbidden(&self) -> Option<(String, String)> {
        Some(("card.move".into(), "c3".into()))
    }
}

async fn run(mode: Mode) -> fs_module::conformance::Report {
    run_conformance(&Fixture(FakeSource::client(mode))).await
}

#[tokio::test(flavor = "current_thread")]
async fn the_strict_fake_passes_every_rule() {
    let r = LocalSet::new().run_until(run(Mode::Strict)).await;
    assert!(r.failures.is_empty(), "{:#?}", r.failures);
    assert!(r.skipped.is_empty(), "{:#?}", r.skipped);
    assert!(r.checked > 30, "{}", r.checked);
}

#[tokio::test(flavor = "current_thread")]
async fn lax_errors_must_turn_the_suite_red() {
    let r = LocalSet::new().run_until(run(Mode::LaxErrors)).await;
    println!("RED: {:#?}", r.failures);
    assert!(
        !r.failures.is_empty(),
        "the suite did not notice a broken error channel"
    );
    assert!(
        r.failures.iter().any(|f| f.starts_with("8 ")),
        "{:#?}",
        r.failures
    );
}
