//! The marks of this program as a `MarkSource` of the generators of `capnp-compiler` (`cligen`, `mcpgen`).
//!
//! Pattern: Adapter — [`ProgramMarks`] answers the questions of the generators (`risk`, `excluded`, `tool_name`,
//! `secret`) from what `fs-mcp` already keeps: the mark files ([`crate::RiskSource`]) and the block list. The keys
//! of the generators (`tasks/Items.create`) and the [`crate::Target`] of the marks are the same text, so nothing is
//! translated except the spelling of the risk classes. The annotations of the schema always count as well; this
//! adapter only adds to them and never takes an exclusion away.
//!
//! What the adapter does not answer: `kind` (a format of the mark files is no basic type), and anything about the
//! results of a command. The screen of this crate ([`crate::Surface`]) covers results.

use capnp_compiler::cligen::MarkSource;

use crate::secrets::name_looks_secret;
use crate::{Blocklist, RiskClass, RiskSource, Target};

/// The marks and the block list of the program, as a source of marks for the generators.
pub struct ProgramMarks<R> {
    risk: R,
    blocklist: Blocklist,
}

impl<R: RiskSource> ProgramMarks<R> {
    /// The adapter over `risk` (usually [`crate::MarkFiles`]) and the block list in force.
    #[must_use]
    pub fn new(risk: R) -> Self {
        Self {
            risk,
            blocklist: Blocklist::standard(),
        }
    }

    /// The same with another block list (tests that must prove the other safeguards hold without it).
    #[must_use]
    pub fn with_blocklist(risk: R, blocklist: Blocklist) -> Self {
        Self { risk, blocklist }
    }
}

/// `tasks/Items.create` as (file, owner, name); the name is empty for a key without a dot.
fn split(key: &str) -> Option<(&str, &str, &str)> {
    let (file, rest) = key.split_once('/')?;
    let (owner, name) = rest.split_once('.').unwrap_or((rest, ""));
    Some((file, owner, name))
}

fn target_of(key: &str) -> Option<Target> {
    let (file, owner, name) = split(key)?;
    Some(if name.is_empty() {
        Target::owner(file, owner)
    } else {
        Target::item(file, owner, name)
    })
}

/// `other-uid` as `otherUid`: the spelling of the contract (the generators spell parameter paths in kebab case).
pub(crate) fn camel(kebab: &str) -> String {
    let mut out = String::new();
    let mut up = false;
    for c in kebab.chars() {
        if c == '-' {
            up = true;
        } else if up {
            out.extend(c.to_uppercase());
            up = false;
        } else {
            out.push(c);
        }
    }
    out
}

impl<R: RiskSource> ProgramMarks<R> {
    fn class(&self, target: &Target) -> Option<RiskClass> {
        self.risk.mark(target).map(|m| m.class)
    }

    /// Is a secret-looking parameter name released in writing: for this very parameter, or for a field of that
    /// name in the same schema file?
    fn released(&self, file: &str, owner: &str, method: &str, name: &str) -> bool {
        if self
            .risk
            .release(&Target::item(file, owner, &format!("{method}.{name}")))
            .is_some()
        {
            return true;
        }
        let prefix = format!("{file}/");
        let suffix = format!(".{name}");
        // a field of a struct is `file/Struct.name` (one dot); `file/Owner.method.name` is a parameter of one method only
        self.risk.targets().iter().any(|t| {
            t.as_str().strip_prefix(&prefix).is_some_and(|rest| {
                rest.ends_with(&suffix)
                    && rest.matches('.').count() == 1
                    && self.risk.release(t).is_some()
            })
        })
    }
}

impl<R: RiskSource> MarkSource for ProgramMarks<R> {
    /// A parameter whose name looks like a secret and has no written release is secret: no tool for the command.
    /// (The comments of the contract are read by the screen of the catalog, [`crate::Surface`].)
    fn secret(&self, key: &str, path: &str) -> bool {
        let Some((file, owner, method)) = split(key) else {
            return false;
        };
        path.split('.')
            .filter(|seg| *seg != "*" && !seg.is_empty())
            .map(camel)
            .any(|name| name_looks_secret(&name) && !self.released(file, owner, method, &name))
    }

    fn risk(&self, key: &str) -> Option<String> {
        let class = self.class(&target_of(key)?)?;
        Some(
            match class {
                RiskClass::Read => "read",
                RiskClass::Add => "add",
                RiskClass::Change => "change",
                RiskClass::Destructive => "destructive",
                RiskClass::ActsInName => "actsInName",
                // an exclusion is `excluded`, the rest is no class the generator knows: unmarked
                RiskClass::Excluded | RiskClass::Unknown => return None,
            }
            .to_owned(),
        )
    }

    /// Excluded: marked so (the command, a navigation method, or the whole interface), a resource (a later
    /// step, never a tool now), or on the block list (the target or its interface).
    fn excluded(&self, key: &str) -> bool {
        let Some((file, owner, _)) = split(key) else {
            return false;
        };
        let Some(target) = target_of(key) else {
            return false;
        };
        let own = Target::owner(file, owner);
        let marked = |t: &Target| {
            self.risk
                .mark(t)
                .is_some_and(|m| m.class == RiskClass::Excluded || m.resource)
        };
        marked(&target)
            || marked(&own)
            || self.blocklist.reason(&target).is_some()
            || self.blocklist.reason(&own).is_some()
            || self.blocklist.type_reason(owner).is_some()
    }

    /// The capability id of the mark, if there is one (`message.send`), else the generator's own name.
    fn tool_name(&self, key: &str) -> Option<String> {
        self.risk.mark(&target_of(key)?)?.capability
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MarkFiles;

    fn marks(text: &str) -> ProgramMarks<MarkFiles> {
        ProgramMarks::new(MarkFiles::from_sources(&[("m", text)]).expect("marks parse"))
    }

    #[test]
    fn kebab_is_turned_back_into_the_contract_spelling() {
        assert_eq!(camel("other-uid"), "otherUid");
        assert_eq!(camel("plain"), "plain");
        assert_eq!(camel("a-b-c"), "aBC");
    }

    #[test]
    fn keys_are_split_like_the_targets() {
        assert_eq!(split("m/Items.create"), Some(("m", "Items", "create")));
        assert_eq!(split("m/Items"), Some(("m", "Items", "")));
        assert_eq!(split("Items.create"), None);
    }

    #[test]
    fn the_risk_is_spelled_the_way_the_annotation_spells_it() {
        let m = marks(
            "[Items]\nr = \"read\"\na = \"add\"\nc = \"change\"\nd = \"destructive\"\nn = \"acts-in-name\"\nx = { risk = \"excluded\", reason = \"admin\" }\nu = \"unknown\"\n",
        );
        let r = |k: &str| m.risk(&format!("m/Items.{k}"));
        assert_eq!(r("r").as_deref(), Some("read"));
        assert_eq!(r("a").as_deref(), Some("add"));
        assert_eq!(r("c").as_deref(), Some("change"));
        assert_eq!(r("d").as_deref(), Some("destructive"));
        assert_eq!(r("n").as_deref(), Some("actsInName"));
        assert_eq!(r("x"), None);
        assert_eq!(r("u"), None);
        assert_eq!(r("nothing"), None);
        assert_eq!(m.risk("no-slash"), None);
    }
}
