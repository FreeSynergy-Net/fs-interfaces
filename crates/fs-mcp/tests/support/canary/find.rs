//! What the canary test found, collected until the end, and the list it writes.
//! Test helpers; no pattern of their own. The test does not stop at the first hit: it lists ALL leaks.
#![allow(dead_code, clippy::format_push_string)] // a text report: pushing formatted lines is what it is for

use std::path::PathBuf;

use fs_mcp::canary::{Canary, CanaryTable, Class, scan};

/// One leak: which canary, where it came out, in which form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub class: Class,
    pub path: String,
    pub marker: String,
    pub place: String,
    pub encoding: String,
    pub found_in: String,
}

/// Everything found, and what was done (so a clean run says what it covered).
#[derive(Default)]
pub struct Findings {
    pub list: Vec<Finding>,
    pub calls: usize,
    pub covered: Vec<String>,
}

impl Findings {
    /// Search `text`, which came from `place`, for every canary of the table.
    pub fn check(&mut self, place: &str, text: &str, table: &CanaryTable) {
        for h in scan(text, table.all()) {
            let f = Finding {
                class: h.class,
                path: h.path,
                marker: h.marker,
                place: place.to_owned(),
                encoding: h.encoding.to_string(),
                found_in: h.found_in,
            };
            if !self.list.contains(&f) {
                self.list.push(f);
            }
        }
    }

    pub fn note(&mut self, what: impl Into<String>) {
        self.covered.push(what.into());
    }

    /// The report: every leak with marker, field path, place and form, then what was covered.
    pub fn report(&self, branch: &str, planted: &[Canary]) -> String {
        let mut s = format!("# Fehlerliste Kanarien-Test ({branch})\n\n");
        s += &format!(
            "{} Abfluesse, {} Aufrufe, {} Kanarien gepflanzt.\n\n",
            self.list.len(),
            self.calls,
            planted.len()
        );
        if self.list.is_empty() {
            s += "Kein Abfluss gefunden.\n\n";
        } else {
            s += "| Klasse | Marker | Feldpfad | Fundort | Kodierung | Sicht |\n|---|---|---|---|---|---|\n";
            for f in &self.list {
                s += &format!(
                    "| {} | {} | {} | {} | {} | {} |\n",
                    f.class.name(),
                    f.marker,
                    f.path,
                    f.place,
                    f.encoding,
                    f.found_in
                );
            }
            s += "\n";
        }
        s += "## Gepflanzt\n\n";
        for c in planted {
            s += &format!("- {} ({}): {}\n", c.path, c.class.name(), c.marker);
        }
        s += "\n## Abgedeckt\n\n";
        for c in &self.covered {
            s += &format!("- {c}\n");
        }
        s
    }

    /// Write the report; `FSCANARY_FEHLERLISTE` names the file, else it goes to the test target.
    pub fn write(&self, branch: &str, planted: &[Canary]) -> PathBuf {
        let path = std::env::var_os("FSCANARY_FEHLERLISTE").map_or_else(
            || {
                PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
                    .join(format!("fehlerliste-kanarien-{}.md", std::process::id()))
            },
            PathBuf::from,
        );
        std::fs::write(&path, self.report(branch, planted)).expect("the list can be written");
        path
    }
}
