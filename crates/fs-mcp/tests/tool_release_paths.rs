//! A tool release names the places it lifts (`paths`): a later hit at another place of the same tool is not released
//! with it, and the format is strict. Pattern: none (tests).

mod support;

use fs_mcp::{MarkFiles, RiskClass, Target, TypeRef, Why};
use support::{FILE, FakeMarks, contract, field, has_tool, interface, method, run, st, strukt};

/// `Items.report` gives a `Report` with two fields whose comments say "secret".
fn two_hits() -> Vec<fs_mcp::Contract> {
    let mut a = field("first", TypeRef::Text);
    a.doc = "no secret here".into();
    let mut b = field("second", TypeRef::Text);
    b.doc = "no secret here either".into();
    contract(
        vec![interface(
            "Items",
            vec![method("report", vec![], vec![field("r", st("Report"))])],
        )],
        vec![strukt("Report", vec![a, b])],
    )
}

fn marks(paths: &[&str]) -> FakeMarks {
    let mut m = FakeMarks::default();
    m.put("Items", "report", RiskClass::Read)
        .release_tool("Items", "report", paths);
    m
}

#[test]
fn a_release_lifts_the_named_places_and_a_hit_somewhere_else_still_keeps_the_tool_out() {
    let c = two_hits();
    let only_first = run(&c, &marks(&["messenger/Report.first"]));
    assert!(!has_tool(&only_first, "Items", "report"));
    assert!(only_first.left_out.iter().any(
        |l| matches!(&l.why, Why::SecretSuspect { path } if path == "messenger/Report.second")
    ));
    let both = run(
        &c,
        &marks(&["messenger/Report.first", "messenger/Report.second"]),
    );
    assert!(has_tool(&both, "Items", "report"));
    assert_eq!(both.unused_releases.len(), 0);
    assert_eq!(both.unused_release_paths.len(), 0);
}

#[test]
fn a_path_that_lifted_nothing_is_a_warning_of_its_own() {
    let c = two_hits();
    let cat = run(
        &c,
        &marks(&[
            "messenger/Report.first",
            "messenger/Report.second",
            "messenger/Report.zzz",
        ]),
    );
    assert!(has_tool(&cat, "Items", "report"));
    assert_eq!(
        cat.unused_releases.len(),
        0,
        "the release as a whole lifted something"
    );
    assert_eq!(
        cat.unused_release_paths,
        [(
            Target::item(FILE, "Items", "report"),
            "messenger/Report.zzz".to_owned()
        )]
    );
}

#[test]
fn the_paths_of_a_release_are_strictly_checked() {
    let entry = |paths: &str| {
        format!("[[tool_release]]\ntool = \"A.b\"\nreason = \"r\"\nsource = \"s\"\n{paths}")
    };
    assert!(MarkFiles::from_sources(&[("f", &entry("paths = [\"f/X.y\", \"f/Z\"]\n"))]).is_ok());
    for bad in [
        "",
        "paths = []\n",
        "paths = \"f/X.y\"\n",
        "paths = [1]\n",
        "paths = [\"\"]\n",
        "paths = [\"no-slash\"]\n",
        "paths = [\"/X.y\"]\n",
        "paths = [\"f/\"]\n",
        "paths = [\"f/X y\"]\n",
        "paths = [\"f/X.y\", \"f/X.y\"]\n",
    ] {
        assert!(
            MarkFiles::from_sources(&[("f", &entry(bad))]).is_err(),
            "{bad}"
        );
    }
    let m = MarkFiles::from_sources(&[("f", &entry("paths = [\"f/X.y\", \"f/Z\"]\n"))]).unwrap();
    let r = fs_mcp::RiskSource::tool_release(&m, &Target::item("f", "A", "b")).unwrap();
    assert_eq!(r.paths, ["f/X.y", "f/Z"]);
}
