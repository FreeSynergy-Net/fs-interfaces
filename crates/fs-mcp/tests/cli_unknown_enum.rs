//! The generated command code reports an enumerant it does not know (a newer writer, an older reader) as
//! `Value::UnknownEnum(number)`: the output carries the number, nothing panics, the call still succeeds.
//! Pattern: none (tests).

use std::future::{Future, ready};

use capnp_compiler::cligen::rt::{CallError, Input, Output, Value};
use fs_interfaces::tasks_capnp::{Readiness, items, readiness_report, tasks};
use fs_mcp::tasks_cli;
use futures::executor::LocalPool;

/// The number of a `Readiness` that no enumerant has (the real ones are 0..=5).
const NEWER: u16 = 99;

/// A report whose readiness is a number this reader does not know (a message from a newer writer).
fn newer_report() -> capnp::message::Reader<capnp::serialize::OwnedSegments> {
    let mut m = capnp::message::Builder::new_default();
    m.init_root::<readiness_report::Builder<'_>>()
        .set_readiness(Readiness::Ready);
    let mut bytes = capnp::serialize::write_message_to_words(&m);
    // Segment table (8 bytes), root pointer (8 bytes), then the data section: `readiness` is its first u16.
    bytes[16..18].copy_from_slice(&NEWER.to_le_bytes());
    let reader =
        capnp::serialize::read_message(&mut &bytes[..], capnp::message::ReaderOptions::new())
            .unwrap();
    let report: readiness_report::Reader<'_> = reader.get_root().unwrap();
    assert!(
        report.get_readiness().is_err(),
        "the test message really carries an unknown enumerant"
    );
    reader
}

struct Items;
struct Root;

impl items::Server for Items {
    fn readiness(
        self: capnp::capability::Rc<Self>,
        _: items::ReadinessParams,
        mut results: items::ReadinessResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        let m = newer_report();
        let report: readiness_report::Reader<'_> = m.get_root().unwrap();
        ready(results.get().set_report(report))
    }
}

impl tasks::Server for Root {
    fn items(
        self: capnp::capability::Rc<Self>,
        _: tasks::ItemsParams,
        mut results: tasks::ItemsResults,
    ) -> impl Future<Output = Result<(), capnp::Error>> + 'static {
        results.get().set_items(capnp_rpc::new_client(Items));
        ready(Ok(()))
    }
}

/// No parameter is given.
struct NoInput;

impl Input for NoInput {
    fn has(&self, _: &str) -> bool {
        false
    }
    fn text(&mut self, _: &str) -> Result<Option<String>, CallError> {
        Ok(None)
    }
    fn int(&mut self, _: &str) -> Result<Option<i128>, CallError> {
        Ok(None)
    }
    fn float(&mut self, _: &str) -> Result<Option<f64>, CallError> {
        Ok(None)
    }
    fn boolean(&mut self, _: &str) -> Result<Option<bool>, CallError> {
        Ok(None)
    }
    fn enumerant(&mut self, _: &str, _: &[&str]) -> Result<Option<usize>, CallError> {
        Ok(None)
    }
    fn count(&mut self, _: &str) -> Result<Option<u32>, CallError> {
        Ok(None)
    }
    fn data(&mut self, _: &str) -> Result<Option<Vec<u8>>, CallError> {
        Ok(None)
    }
}

/// Keeps what the code reports about enumerants.
#[derive(Default)]
struct Seen(Vec<(String, String)>);

impl Output for Seen {
    fn begin_struct(&mut self, _: &str, _: &str) {}
    fn begin_list(&mut self, _: &str, _: &str) {}
    fn field(&mut self, name: &str, value: Value<'_>) {
        if let Value::UnknownEnum(_) | Value::Enum(_) = value {
            self.0.push((name.to_owned(), format!("{value:?}")));
        }
    }
    fn end(&mut self) {}
}

#[test]
fn an_unknown_enumerant_is_reported_with_its_number_and_nothing_panics() {
    let client: tasks::Client = capnp_rpc::new_client(Root);
    let mut seen = Seen::default();
    let mut pool = LocalPool::new();
    let mut input = NoInput;
    let run = tasks_cli::run_items_readiness(&client, &mut input, &mut seen);
    let outcome = pool.run_until(run);
    assert!(outcome.is_ok(), "the call succeeds: {:?}", outcome.err());
    assert_eq!(
        seen.0,
        [("readiness".to_owned(), format!("UnknownEnum({NEWER})"))]
    );
}
