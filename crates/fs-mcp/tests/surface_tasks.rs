//! The generated tool table of `tasks.capnp` against the screen of this crate: what a connection may see, and why
//! the rest is out. Pattern: none (tests).

mod support;

use capnp_compiler::mcpgen::rt::{Danger, META_KEY, Never, Off, Switch, ToolSpec};
use fs_mcp::{Refusal, RiskClass, Surface, Target, Why, tasks_cli, tasks_mcp};
use serde_json::Value;
use support::real::{marks, real_catalog};

/// A switch that is on (the program's own, in a real run).
struct On;
impl Switch for On {
    fn is_on(&self) -> bool {
        true
    }
}

fn table() -> &'static [ToolSpec] {
    tasks_mcp::tools(&On)
}

#[test]
fn off_and_the_vault_show_nothing() {
    let catalog = real_catalog();
    let surface = Surface::new(&catalog);
    assert_eq!(tasks_mcp::tools(&Off).len(), 0);
    assert_eq!(tasks_mcp::tools(&Never).len(), 0);
    for s in [&Off as &dyn Switch, &Never] {
        let a = surface.admit(s, tasks_mcp::TOOLS);
        assert!(
            a.offered.is_empty() && a.refused.is_empty(),
            "an off switch lists nothing, not even refusals"
        );
    }
    const { assert!(!tasks_mcp::ENABLED_BY_DEFAULT) };
}

#[test]
fn every_tool_is_offered_or_refused_with_a_reason_and_the_table_is_complete() {
    let catalog = real_catalog();
    let a = Surface::new(&catalog).admit(&On, tasks_mcp::TOOLS);
    assert_eq!(a.offered.len() + a.refused.len(), tasks_mcp::TOOLS.len());
    // with the written releases of the mark files: 19 of 20. `tasks.types.describe` has a SECOND hit at another place
    // (`FieldDescription.label`, "translation key") that its release does not name; it stays out until a human decides.
    assert_eq!(a.offered.len(), 19);
    let refused: Vec<_> = a.refused.iter().map(|(t, r)| (t.name, *r)).collect();
    assert_eq!(refused, [("tasks.types.describe", Refusal::NotInCatalog)]);
    // the table is in walk order and unique
    let mut names: Vec<&str> = tasks_mcp::TOOLS.iter().map(|t| t.name).collect();
    let n = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), n, "tool names are unique");
}

#[test]
fn what_is_offered_is_in_the_catalog_and_the_catalog_decides() {
    let catalog = real_catalog();
    let a = Surface::new(&catalog).admit(&On, tasks_mcp::TOOLS);
    for t in &a.offered {
        let (file, rest) = t.key.split_once('/').unwrap();
        let (owner, name) = rest.split_once('.').unwrap();
        assert!(
            catalog
                .tools
                .iter()
                .any(|c| c.source == Target::item(file, owner, name)),
            "{} is offered but the catalog has no tool for {}",
            t.name,
            t.key
        );
    }
    // a refusal for the catalog is exactly "no tool in the catalog"
    for (t, why) in &a.refused {
        assert_eq!(*why, Refusal::NotInCatalog, "{}", t.name);
    }
}

#[test]
fn every_table_tool_says_ai_and_carries_the_class_of_the_marks() {
    let marks = marks();
    for t in table() {
        let meta: Value = serde_json::from_str(t.meta).expect("meta is JSON");
        assert_eq!(meta[META_KEY]["actor"], "ai", "{}", t.name);
        let (file, rest) = t.key.split_once('/').unwrap();
        let (owner, name) = rest.split_once('.').unwrap();
        let mark = fs_mcp::RiskSource::mark(&marks, &Target::item(file, owner, name))
            .unwrap_or_else(|| panic!("no mark for {}", t.key));
        let want = match mark.class {
            RiskClass::Read => Danger::Read,
            RiskClass::Add => Danger::Add,
            RiskClass::Change => Danger::Change,
            RiskClass::Destructive => Danger::Destructive,
            RiskClass::ActsInName => Danger::ActsInName,
            other => panic!("{}: {other:?} must not be a tool", t.key),
        };
        assert_eq!(t.risk, Some(want), "{}", t.key);
        assert_eq!(meta[META_KEY]["unmarked"], false, "{}", t.key);
    }
}

#[test]
fn the_dangerous_ones_are_labelled_so() {
    let by = |n: &str| tasks_mcp::TOOLS.iter().find(|t| t.name == n).unwrap();
    assert_eq!(by("tasks.items.delete").risk, Some(Danger::Destructive));
    assert_eq!(by("tasks.items.get").risk, Some(Danger::Read));
    assert_eq!(by("tasks.items.assign").risk, Some(Danger::ActsInName));
}

/// Every property name that an offered tool takes or gives (both schemas, all of `$defs`).
fn property_names(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            if let Some(Value::Object(p)) = m.get("properties") {
                out.extend(p.keys().cloned());
            }
            m.values().for_each(|x| property_names(x, out));
        }
        Value::Array(a) => a.iter().for_each(|x| property_names(x, out)),
        _ => {}
    }
}

#[test]
fn no_offered_tool_takes_or_gives_a_field_that_looks_like_a_secret() {
    let catalog = real_catalog();
    let a = Surface::new(&catalog).admit(&On, tasks_mcp::TOOLS);
    let mut names = Vec::new();
    for t in &a.offered {
        for text in [t.input_schema, t.output_schema] {
            property_names(
                &serde_json::from_str(text).expect("schema is JSON"),
                &mut names,
            );
        }
    }
    assert_ne!(names.len(), 0);
    for n in names {
        assert!(
            !fs_mcp::name_looks_secret(&n),
            "`{n}` looks like a secret and an offered tool carries it"
        );
    }
}

#[test]
fn the_tools_find_their_command_code() {
    // every offered tool has a function in the command code of the same request
    for t in table() {
        assert!(
            tasks_mcp::runner(t).is_some(),
            "{} has no command: {:?}",
            t.name,
            t.words
        );
        assert!(
            tasks_cli::COMMANDS.iter().any(|c| c.key == t.key),
            "{}",
            t.key
        );
    }
}

#[test]
fn the_left_list_explains_what_is_no_tool() {
    assert!(
        tasks_mcp::LEFT
            .iter()
            .any(|(k, why)| *k == "tasks/Tasks.subscribe" && *why == "listener")
    );
    for (k, why) in tasks_mcp::LEFT {
        assert!(
            !tasks_mcp::TOOLS.iter().any(|t| t.key == *k),
            "{k} ({why}) is in both lists"
        );
    }
}

/// The marks without their tool releases: the state the screen alone gives.
struct NoReleases(fs_mcp::MarkFiles);
impl fs_mcp::RiskSource for NoReleases {
    fn mark(&self, t: &Target) -> Option<fs_mcp::Mark> {
        self.0.mark(t)
    }
    fn release(&self, t: &Target) -> Option<String> {
        self.0.release(t)
    }
    fn format(&self, t: &Target) -> Option<String> {
        self.0.format(t)
    }
    fn targets(&self) -> Vec<Target> {
        self.0.targets()
    }
}

#[test]
fn without_the_releases_exactly_two_tools_are_refused_each_with_its_reason() {
    use fs_mcp::{Blocklist, ToolGenerator};
    let no = NoReleases(marks());
    let g = fs_mcp::Generator::new(&no, Blocklist::standard());
    let catalog = g.generate(&support::real::contracts()).unwrap();
    let a = Surface::new(&catalog).admit(&On, tasks_mcp::TOOLS);
    assert_eq!(a.offered.len(), 18);
    let refused: Vec<_> = a.refused.iter().map(|(t, r)| (t.name, *r)).collect();
    assert_eq!(
        refused,
        [
            ("tasks.support-report", Refusal::NotInCatalog),
            ("tasks.types.describe", Refusal::NotInCatalog)
        ]
    );
    let why = |target: &str| {
        catalog
            .left_out
            .iter()
            .find(|l| l.target.as_str() == target)
            .map(|l| l.why.clone())
    };
    // the comment on `instance` (common.capnp:122-124) hangs on `entries` and says "secret"
    assert_eq!(
        why("tasks/Tasks.supportReport"),
        Some(Why::SecretSuspect {
            path: "common/SupportReport.entries".to_owned()
        })
    );
    // "translation key" in the comment of `TypeDescription.label` (fs-types)
    assert_eq!(
        why("types/TypeCatalog.describe"),
        Some(Why::SecretSuspect {
            path: "types/TypeDescription.label".to_owned()
        })
    );
}
