//! The tools a connection may see: the generated table, cut down by the switch and by the screen of this crate.
//!
//! Pattern: Specification — a tool is offered only if every predicate holds: the switch is on (the table is empty
//! while it is off, and for the vault, which is `Never`), the tool is not marked excluded, it says in its `_meta`
//! that an AI acts (`"actor":"ai"`), and the catalog of this crate ([`crate::Catalog`]) also offers it. The last
//! one is the mandatory intersection: the generator of `capnp-compiler` knows the annotations of the schema (a
//! `secret` field, `risk(excluded)`), this crate knows the marks and the screen (names and comments that look like
//! a secret, free text, `AnyPointer`, the block list) and reads the results of a command as well, which the marks of
//! a program do not reach in the generator. A tool that only one of them lets through is out. Nothing here runs a
//! call or speaks the protocol (stage 2, the server, uses the answer).
//!
//! The result of a call never carries a field of a tool that is not offered here: there is no tool, so there is no
//! call and no `structuredContent` or text block.

use capnp_compiler::mcpgen::rt::{Danger, META_KEY, Switch, ToolSpec, visible};
use serde_json::Value;

use crate::Catalog;

/// Why a tool of the table is not offered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The catalog of this crate does not offer it (a secret-looking name, free text, a blocked target, ...).
    NotInCatalog,
    /// The table itself marks it as excluded.
    Excluded,
    /// Its `_meta` does not say `"actor":"ai"`, or is not the JSON object that every generated table has.
    NoAiLabel,
}

/// What the screen made of a table.
#[derive(Debug, PartialEq, Eq)]
pub struct Admission<'t> {
    /// The tools that may be listed and called, in table order (the table is in walk order, so the order is stable).
    pub offered: Vec<&'t ToolSpec>,
    /// The tools that were left out, with the first reason.
    pub refused: Vec<(&'t ToolSpec, Refusal)>,
}

/// The screen: a catalog, and the rules above.
pub struct Surface<'c> {
    catalog: &'c Catalog,
}

impl<'c> Surface<'c> {
    /// The screen over the catalog of the same contracts and marks that the table was generated from.
    #[must_use]
    pub fn new(catalog: &'c Catalog) -> Self {
        Self { catalog }
    }

    /// Is the table's tool also a tool of the catalog? A method is matched by its key; a union member by the
    /// schema file and the member (the catalog marks a member at the union, not at the method).
    fn in_catalog(&self, spec: &ToolSpec) -> bool {
        let file = spec.key.split_once('/').map_or("", |(f, _)| f);
        self.catalog.tools.iter().any(|t| {
            let source = t.source.as_str();
            match spec.member {
                None => source == spec.key,
                Some(m) => {
                    source.split_once('/').is_some_and(|(f, _)| f == file)
                        && source.ends_with(&format!(".{m}"))
                }
            }
        })
    }

    /// The first reason to keep `spec` out, if there is one.
    #[must_use]
    pub fn refusal(&self, spec: &ToolSpec) -> Option<Refusal> {
        if spec.risk == Some(Danger::Excluded) {
            return Some(Refusal::Excluded);
        }
        if !marked_ai(spec.meta) {
            return Some(Refusal::NoAiLabel);
        }
        (!self.in_catalog(spec)).then_some(Refusal::NotInCatalog)
    }

    /// The tools of `table` that `switch` lets through and the screen admits. An off switch gives nothing at all
    /// (also no list of refusals: a vault says nothing about what it would have held).
    #[must_use]
    pub fn admit<'t>(&self, switch: &dyn Switch, table: &'t [ToolSpec]) -> Admission<'t> {
        let mut out = Admission {
            offered: Vec::new(),
            refused: Vec::new(),
        };
        for spec in visible(switch, table) {
            match self.refusal(spec) {
                None => out.offered.push(spec),
                Some(r) => out.refused.push((spec, r)),
            }
        }
        out
    }
}

/// Does the `_meta` text say `"actor":"ai"` under our key?
fn marked_ai(meta: &str) -> bool {
    serde_json::from_str::<Value>(meta)
        .ok()
        .and_then(|v| v.get(META_KEY)?.get("actor")?.as_str().map(|a| a == "ai"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_label_is_read_from_the_meta_and_only_the_value_ai_counts() {
        let key = META_KEY;
        assert!(marked_ai(&format!("{{\"{key}\":{{\"actor\":\"ai\"}}}}")));
        assert!(!marked_ai(&format!(
            "{{\"{key}\":{{\"actor\":\"human\"}}}}"
        )));
        assert!(!marked_ai(&format!("{{\"{key}\":{{}}}}")));
        assert!(!marked_ai(&format!("{{\"{key}\":{{\"actor\":true}}}}")));
        assert!(!marked_ai("{\"other\":{\"actor\":\"ai\"}}"));
        assert!(!marked_ai("not json"));
        assert!(!marked_ai("[]"));
    }
}
