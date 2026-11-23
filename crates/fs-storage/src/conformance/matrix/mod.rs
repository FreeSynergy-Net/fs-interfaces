//! The rights matrix as a table: every row runs for every actor on a fresh world and must give exactly the expected result.
//!
//! Pattern: Table-driven test (data in `rows_*.rs`, one driver in `exec.rs`). THIS TABLE is the source of the exact result
//! codes (ADR 0070, rights matrix, with the strategy decisions F1 to F5); `rows_*.rs` replace the old scratch file. The
//! older rule sweeps (`none_allowed`, `own_into_foreign`, `stranger_*`, `member_readonly`, `rights.bot`) are "every
//! instance" checks derived from it: they try EVERY entry of the tree and only expect "Forbidden, tree unchanged". Both
//! must stay green on the same fake, so changing one without the other turns the suite red, never silently green.
//! The column `faults` names the faults (mutants) that break exactly this row; `every_named_fault_breaks_its_row`
//! proves each claim.

mod exec;
mod methods;
mod model;
mod rows;
mod rows_a;
mod rows_b;
mod setup;

pub(super) use exec::run_rows;
pub use methods::{METHODS, Method};
pub use model::{ACTORS, Actor, Expect, Op, Row, Target};
pub use rows::{COVERED_BY_RULES, rows};
