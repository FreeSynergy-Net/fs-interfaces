//! The canary test (feature `conformance`, in no `default`): plant markers where a secret would sit, run everything a client
//! can run, and search every answer, error text and log for them in whatever form they come back.
//!
//! Pattern: Specification (the search) over Value objects (the markers). The driver that plants and calls is a test of the
//! program (see `tests/`); this module is only the part that does not depend on a program: the markers and `scan`.

mod marker;
mod scan;
mod views;

pub use marker::{
    Canary, CanaryTable, Class, PREFIX, PREFIX_SENSITIVE, RANDOM_LEN, class_of, fresh_marker,
    fresh_marker_of, is_marker,
};
pub use scan::{ANY, Encoding, Hit, WINDOW, scan};
