//! Test providers and the fixed sample data of FreeSynergy calendar (pilot `programm-grundgeruest`, step K0/F0).
//!
//! DRAFT: written against the not yet locked `Appointments` trait draft ([`draft`], state after the closing of ADR 0085, 2026-11-16). The frontend develops
//! against [`FakeAppointments`]; the core agent owns this crate.
//!
//! * [`FakeAppointments`]: in memory, every method, with a trash (period, retention), calendars, feeds; [`EmptyAppointments`] is the empty one (Z1).
//! * [`SlowAppointments`]: any provider, delayed (Z2). [`FailingAppointments`]: always `Unavailable` (Z3).
//! * [`FakePeople`]: the consumed person directory (the form of `person_directory_v1`, a small own port until fs-people has a tag).
//! * [`sample`]: the fixed set of appointments (every case of the plan, section 5).

pub mod cal_feed;
pub mod cal_manage;
pub mod check;
pub mod conformance;
pub mod draft;
pub mod failing;
pub mod fake;
pub mod freebusy;
pub mod ids;
pub mod instances;
pub mod patch;
pub mod people;
pub mod provider;
pub mod purge;
pub mod read;
pub mod recur;
pub mod sample;
pub mod sample_cals;
pub mod sample_items;
pub mod sample_people;
pub mod scope;
pub mod slow;
pub mod social;
pub mod ssrf;
pub mod state;
pub mod trash;
pub mod trashing;
pub mod write;
pub mod zone;

pub use failing::FailingAppointments;
pub use fake::{DECISIONS, EmptyAppointments, FakeAppointments};
pub use people::FakePeople;
pub use sample::{Sample, sample};
pub use slow::SlowAppointments;
