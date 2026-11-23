//! The rights matrix as a table: every row runs for every actor on a fresh world and must give exactly the expected result.
//!
//! Pattern: Table-driven test (data in `rows_*.rs`, one driver in `exec.rs`). Source: RECHTE-MATRIX.md (Opus, 23.11.2026)
//! with the strategy decisions F1 to F5. A cell with `?` is provisional ("Strategie-Entscheid vorläufig").

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
