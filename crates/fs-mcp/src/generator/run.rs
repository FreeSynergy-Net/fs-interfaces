//! The run: [`Generator`] and the walk over interfaces that fills a catalog. It runs nothing.
//!
//! Pattern: Strategy — [`Generator`] implements [`ToolGenerator`] and takes its marks from any
//! [`RiskSource`] (mark files now, annotations later). The walk over interfaces fills a catalog; every
//! candidate must pass `vet` (marks) and the screen (types) before it becomes a tool or resource.
//! Trait: [`ToolGenerator`].

use std::cell::RefCell;
use std::collections::HashSet;

use super::api::ToolGenerator;
use super::facts::{self, Facts};
use super::screen::Screen;
use crate::{Blocklist, Catalog, Contract, Error, Index, Interface, Left, RiskSource, Target, Why};

/// The stage 1 generator.
pub struct Generator<'a> {
    pub(super) risk: &'a dyn RiskSource,
    pub(super) blocklist: Blocklist,
}

impl<'a> Generator<'a> {
    /// A generator reading marks from `risk`. The block list wins over every mark.
    #[must_use]
    pub fn new(risk: &'a dyn RiskSource, blocklist: Blocklist) -> Self {
        Self { risk, blocklist }
    }
}

/// State of one run.
pub(crate) struct Run<'a> {
    pub(crate) g: &'a Generator<'a>,
    pub(crate) index: Index<'a>,
    pub(crate) facts: Facts,
    pub(crate) out: Catalog,
    pub(crate) errors: Vec<Error>,
    /// The paths the screen lifted a hit at (for the candidate being vetted).
    pub(crate) lifted: RefCell<HashSet<String>>,
    /// For every tool whose release lifted something: the paths it lifted.
    pub(crate) used_releases: std::collections::HashMap<Target, HashSet<String>>,
}

impl Run<'_> {
    pub(super) fn screen(&self) -> Screen<'_> {
        self.screen_with(None)
    }

    /// The screen of one candidate with its written tool release for comment-word hits, if it has one.
    pub(super) fn screen_with<'s>(
        &'s self,
        doc_release: Option<&'s crate::ToolRelease>,
    ) -> Screen<'s> {
        Screen {
            risk: self.g.risk,
            blocklist: &self.g.blocklist,
            index: &self.index,
            doc_release,
            used: &self.lifted,
        }
    }

    pub(crate) fn leave(&mut self, target: Target, why: Why) {
        self.out.left_out.push(Left { target, why });
    }

    fn interface(&mut self, file: &str, iface: &Interface) {
        let owner = Target::owner(file, &iface.name);
        if let Some(why) = self.g.blocklist.reason(&owner) {
            return self.leave(owner, Why::Blocked(why));
        }
        if let Some(why) = self.screen().interface(file, iface) {
            return self.leave(owner, why);
        }
        if self.facts.is_callback(file, &iface.name) {
            return self.leave(
                owner,
                Why::NotACall("a listener: the caller hands it in, events become a resource"),
            );
        }
        let root = !self.facts.is_returned(file, &iface.name);
        for m in &iface.methods {
            self.method(file, iface, m, root);
        }
    }
}

impl ToolGenerator for Generator<'_> {
    fn generate(&self, contracts: &[Contract]) -> Result<Catalog, Error> {
        let mut run = Run {
            g: self,
            index: Index::new(contracts),
            facts: Facts::collect(contracts),
            out: Catalog::default(),
            errors: Vec::new(),
            lifted: RefCell::new(HashSet::new()),
            used_releases: std::collections::HashMap::new(),
        };
        for c in contracts {
            for i in &c.interfaces {
                run.interface(&c.file, i);
            }
        }
        if let Some(e) = run.errors.into_iter().next() {
            return Err(e);
        }
        let mut out = run.out;
        let used = run.used_releases;
        let mut seen = HashSet::new();
        for t in &out.tools {
            if !seen.insert(t.name.clone()) {
                return Err(Error::DuplicateName(t.name.clone()));
            }
        }
        let mut uris = HashSet::new();
        for r in &out.resources {
            if !uris.insert(r.uri_template.clone()) {
                return Err(Error::DuplicateName(r.uri_template.clone()));
            }
        }
        out.tools.sort_by(|a, b| a.name.cmp(&b.name));
        out.resources
            .sort_by(|a, b| a.uri_template.cmp(&b.uri_template));
        out.left_out.sort_by(|a, b| a.target.cmp(&b.target));
        out.unmarked.sort();
        out.unmarked.dedup();
        out.stale = facts::stale(contracts, self.risk);
        for t in self.risk.targets() {
            let Some(rel) = self.risk.tool_release(&t) else {
                continue;
            };
            match used.get(&t) {
                None => out.unused_releases.push(t),
                Some(lifted) => {
                    for p in rel.paths.iter().filter(|p| !lifted.contains(*p)) {
                        out.unused_release_paths.push((t.clone(), p.clone()));
                    }
                }
            }
        }
        out.unused_releases.sort();
        out.unused_releases.dedup();
        out.unused_release_paths.sort();
        out.unused_release_paths.dedup();
        Ok(out)
    }
}
