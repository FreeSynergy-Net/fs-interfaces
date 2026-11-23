//! Conformance rules of `appointments_v1`, runnable against any provider (the fake, `LocalAppointments`, later accounts).
//!
//! Pattern: Template method (a fixed list of rules, each one checked against the `Appointments` under test).
//!
//! `run_conformance` takes a [`ConformanceFixture`]: the provider plus the few things the trait cannot do (move the
//! clock, set the trash period, keep an object, switch who asks, say what a feed delivers). It seeds its own data
//! through the trait (every calendar and appointment is named `Zzconf ...`), so it works on an empty or a filled
//! provider. A mutator a fixture cannot offer keeps its default and the rules that need it are reported as skipped,
//! not as passed.

// The rules are flat lists of checks on a few objects; splitting a list hides nothing, hence these lints are off.
#![allow(clippy::too_many_lines, clippy::many_single_char_names)]

mod cx;
mod fixture;
mod rules_ai;
mod rules_calendars;
mod rules_deadline;
mod rules_freebusy;
mod rules_original;
mod rules_purge;
mod rules_subscribe;

mod run;

pub use cx::Report;
pub use fixture::{Advance, Asker, ConformanceFixture};
pub use run::run_conformance;
