//! All rows of the table and the methods that named rules cover instead.
//!
//! Pattern: Registry.

use super::methods::Method;
use super::model::Row;
use super::{rows_a, rows_b};

/// All rows of the table.
#[must_use]
pub fn rows() -> Vec<Row> {
    let mut v = rows_a::ROWS.to_vec();
    v.extend_from_slice(rows_b::ROWS);
    v
}

/// Methods that are not in the table because a named rule covers them (each must be a rule of `run`, see the meta test).
pub const COVERED_BY_RULES: &[(Method, &[&str])] = &[
    (Method::Capabilities, &["caps.honest"]),
    (Method::State, &["caps.honest"]),
    (Method::Roots, &["roots.shape", "rights.manager.no-server"]),
    (Method::Watch, &["caps.honest"]),
    (Method::Scope, &["scope.generator"]),
    (Method::Changes, &["changes.delta", "operator.log"]),
    (
        Method::ResolvePath,
        &["path.traversal", "link.never-followed"],
    ),
    (Method::BlobEntry, &["blob.range"]),
    (Method::BlobRead, &["blob.range", "file.sha256"]),
];
