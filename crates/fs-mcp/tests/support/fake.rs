//! The test double of the risk source. Pattern: Strategy (a second `RiskSource`).
#![allow(dead_code)]

use std::collections::HashMap;

use fs_mcp::{Mark, RiskClass, RiskSource, Target, ToolRelease};

use super::FILE;

/// Marks held in memory: the test double of [`RiskSource`].
#[derive(Default)]
pub struct FakeMarks {
    pub marks: HashMap<Target, Mark>,
    pub releases: HashMap<Target, String>,
    pub formats: HashMap<Target, String>,
    pub tool_releases: HashMap<Target, ToolRelease>,
}

impl FakeMarks {
    pub fn put(&mut self, owner: &str, item: &str, class: RiskClass) -> &mut Self {
        self.marks.insert(
            Target::item(FILE, owner, item),
            Mark {
                class,
                capability: None,
                resource: false,
                reason: None,
            },
        );
        self
    }
    pub fn capability(&mut self, owner: &str, item: &str, class: RiskClass, id: &str) -> &mut Self {
        let m = Mark {
            class,
            capability: Some(id.into()),
            resource: false,
            reason: None,
        };
        self.marks.insert(Target::item(FILE, owner, item), m);
        self
    }
    /// Every method and member of `messenger_like` marked `read`, even the forbidden ones.
    pub fn everything_read() -> Self {
        let mut m = Self::default();
        for (o, i) in [
            ("Messenger", "capabilities"),
            ("Messenger", "conversations"),
            ("Messenger", "subscribe"),
            ("Messenger", "guard"),
            ("Messenger", "draft"),
            ("Conversations", "open"),
            ("Guard", "allow"),
            ("Accounts", "add"),
            ("MessageAction", "none"),
            ("MessageAction", "delete"),
            ("MessageAction", "flag"),
            ("MessageAction", "decide"),
            ("Draft", "send"),
        ] {
            m.put(o, i, RiskClass::Read);
        }
        m
    }
}

impl FakeMarks {
    /// A written tool release for `owner.method`, for the given places.
    pub fn release_tool(&mut self, owner: &str, method: &str, paths: &[&str]) -> &mut Self {
        self.tool_releases.insert(
            Target::item(FILE, owner, method),
            ToolRelease {
                reason: "false alarm".into(),
                source: "test".into(),
                paths: paths.iter().map(|p| (*p).to_owned()).collect(),
            },
        );
        self
    }
}

impl RiskSource for FakeMarks {
    fn tool_release(&self, t: &Target) -> Option<ToolRelease> {
        self.tool_releases.get(t).cloned()
    }
    fn mark(&self, t: &Target) -> Option<Mark> {
        self.marks.get(t).cloned()
    }
    fn release(&self, t: &Target) -> Option<String> {
        self.releases.get(t).cloned()
    }
    fn format(&self, t: &Target) -> Option<String> {
        self.formats.get(t).cloned()
    }
    fn targets(&self) -> Vec<Target> {
        self.marks
            .keys()
            .chain(self.releases.keys())
            .chain(self.tool_releases.keys())
            .cloned()
            .collect()
    }
}
