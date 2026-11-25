//! The runners shared by the suite tests (the fake in this process). The fixture itself is `fs_module::FakeFixture`.
#![allow(dead_code)]

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
