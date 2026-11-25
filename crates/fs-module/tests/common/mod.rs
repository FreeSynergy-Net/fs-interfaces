//! The runners shared by the suite tests (the fake in this process). The fixture itself is `fs_module::FakeFixture`.
#![allow(dead_code)]

use fs_interfaces::module::{
    Actor, Caller, Claimed, Context, Invocation, PageQuery, StepRef, Value,
};
use fs_interfaces::owner::fake::person;
use fs_module::conformance::run_conformance;
use fs_module::{FakeFixture, FakeSource, Mode};

pub async fn run(mode: Mode, blob: bool, boards: usize) -> fs_module::conformance::Report {
    run_level(mode, blob, boards, 2).await
}

/// The fake says `says` in `hello`; the fixture says `claims` (what the suite believes of the service).
pub async fn run_with(mode: Mode, says: u32, claims: u32) -> fs_module::conformance::Report {
    let fake = FakeSource::new(mode).with_level(says);
    let two = capnp_rpc::new_client(fake.connection());
    let fx = FakeFixture::new(capnp_rpc::new_client(fake))
        .with_second(two)
        .with_level(claims);
    run_conformance(&fx).await
}

pub async fn run_level(
    mode: Mode,
    blob: bool,
    boards: usize,
    level: u32,
) -> fs_module::conformance::Report {
    let mut fake = FakeSource::new(mode).with_boards(boards).with_level(level);
    if blob {
        fake = fake.with_blob();
    }
    let two = capnp_rpc::new_client(fake.connection());
    run_conformance(
        &FakeFixture::new(capnp_rpc::new_client(fake))
            .with_second(two)
            .with_level(level)
            .with_blob(blob)
            .with_boards(boards),
    )
    .await
}

// Helpers of the `FakeCore` tests.

pub fn cx(program: &str) -> Context {
    let caller = Caller::Connection {
        system_id: "1000".into(),
        system_name: None,
    };
    Context::from_connection(
        caller,
        Actor::Person,
        person("urn:uuid:018f3a52-7c1e-7b3a-9d4e-5a6b7c8d9e01"),
    )
    .with_claimed(Claimed::new(program, "i1", "d1"))
}

pub fn call(command: &str, record: &str, args: &[(&str, &str)]) -> Invocation {
    let mut c = Invocation::default();
    c.command = command.into();
    c.record = (!record.is_empty()).then(|| record.to_owned());
    c.args = args.iter().map(|(p, t)| Value::new(p, t)).collect();
    c
}

pub fn with_step(mut c: Invocation, run: &str, step: &str) -> Invocation {
    c.step = Some(StepRef::new(run, step));
    c
}

pub fn page_query(form: &str, scope: &str, cursor: &str, limit: u32) -> PageQuery {
    let mut q = PageQuery::default();
    q.form = form.into();
    q.scope = scope.into();
    q.cursor = cursor.into();
    q.limit = limit;
    q
}

pub const FORM: &str = "cards-in-columns@1.0.0";
