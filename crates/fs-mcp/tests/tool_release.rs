//! The written release of ONE tool for comment-word hits of the screen: visible in the mark files, revocable, and weaker
//! than every explicit mark. Pattern: none (tests).

mod support;

use capnp_compiler::mcpgen::rt::{Off, Switch};
use fs_mcp::{MarkFiles, RiskClass, Surface, Target, TypeRef, Why, tasks_mcp};
use support::fx;
use support::real::real_catalog;
use support::{FILE, FakeMarks, contract, field, has_tool, interface, method, run, strukt};

struct On;
impl Switch for On {
    fn is_on(&self) -> bool {
        true
    }
}

/// A tool `Items.report` whose result struct has a comment that says "secret", and the field `plain`.
fn commented(name_hit: bool) -> Vec<fs_mcp::Contract> {
    let mut f = field(if name_hit { "apiToken" } else { "plain" }, TypeRef::Text);
    f.doc = "The id is no secret, only a label".into();
    contract(
        vec![interface(
            "Items",
            vec![
                method("report", vec![], vec![field("r", support::st("Report"))]),
                method("clean", vec![], vec![field("n", TypeRef::Text)]),
            ],
        )],
        vec![strukt("Report", vec![f])],
    )
}

fn marks(release: &[&str]) -> FakeMarks {
    let mut m = FakeMarks::default();
    m.put("Items", "report", RiskClass::Read)
        .put("Items", "clean", RiskClass::Read);
    for r in release {
        m.release_tool("Items", r, &["messenger/Report.plain"]);
    }
    m
}

#[test]
fn without_a_release_the_comment_word_keeps_the_tool_out_and_with_it_the_tool_is_offered() {
    let c = commented(false);
    let off = run(&c, &marks(&[]));
    assert!(!has_tool(&off, "Items", "report"));
    assert!(
        off.left_out.iter().any(
            |l| matches!(&l.why, Why::SecretSuspect { path } if path.ends_with("Report.plain"))
        )
    );
    let on = run(&c, &marks(&["report"]));
    assert!(has_tool(&on, "Items", "report"));
    // only that tool: the release is for one tool
    assert!(has_tool(&on, "Items", "clean"));
    assert_eq!(on.unused_releases.len(), 0);
}

#[test]
fn a_release_without_a_hit_is_a_warning() {
    let c = commented(false);
    // `clean` has no hit: its release lifted nothing
    let cat = run(&c, &marks(&["report", "clean"]));
    assert_eq!(cat.unused_releases, [Target::item(FILE, "Items", "clean")]);
    // and a release for a tool that does not exist is stale
    let cat = run(&c, &marks(&["nothing"]));
    assert_eq!(
        cat.unused_releases,
        [Target::item(FILE, "Items", "nothing")]
    );
    assert!(cat.stale.contains(&Target::item(FILE, "Items", "nothing")));
}

#[test]
fn a_release_lifts_comment_words_only_never_a_secret_name() {
    let c = commented(true);
    let cat = run(&c, &marks(&["report"]));
    assert!(
        !has_tool(&cat, "Items", "report"),
        "`apiToken` is a name, not a comment word"
    );
    assert!(cat.left_out.iter().any(
        |l| matches!(&l.why, Why::SecretSuspect { path } if path.ends_with("Report.apiToken"))
    ));
}

#[test]
fn a_release_never_lifts_an_exclusion_or_the_block_list() {
    let c = commented(false);
    let mut m = marks(&["report"]);
    m.put("Items", "report", RiskClass::Excluded);
    assert!(!has_tool(&run(&c, &m), "Items", "report"));
    // the block list: `messenger/MessageAction.decide` stays out whatever is released
    let mut b = FakeMarks::default();
    b.put("MessageAction", "decide", RiskClass::Read)
        .release_tool(
            "MessageAction",
            "decide",
            &["messenger/MessageAction.decide"],
        );
    let cat = run(&support::messenger_like(), &b);
    assert!(!has_tool(&cat, "MessageAction", "decide"));
}

#[test]
fn an_explicit_secret_annotation_beats_a_release() {
    // `issue` has `$secret` on its result: the generator leaves it out of the table, a release in the marks changes nothing
    let text = "[Notes]\nissue = \"read\"\nget = \"read\"\n[[tool_release]]\ntool = \"Notes.issue\"\nreason = \"x\"\nsource = \"y\"\npaths = [\"fx/Notes.issue\"]\n";
    let marks = fs_mcp::ProgramMarks::new(MarkFiles::from_sources(&[("fx", text)]).unwrap());
    let table = fx::table(marks);
    assert!(!fx::has_tool(&table, "fx.notes.issue"));
    assert_eq!(
        fx::left(&table, "fx/Notes.issue").as_deref(),
        Some("secret")
    );
    assert!(fx::has_tool(&table, "fx.notes.get"));
}

#[test]
fn the_mark_file_format_is_strict() {
    let ok =
        "[[tool_release]]\ntool = \"A.b\"\nreason = \"r\"\nsource = \"s\"\npaths = [\"f/X.y\"]\n";
    let m = MarkFiles::from_sources(&[("f", ok)]).unwrap();
    let r = fs_mcp::RiskSource::tool_release(&m, &Target::item("f", "A", "b")).unwrap();
    assert_eq!((r.reason.as_str(), r.source.as_str()), ("r", "s"));
    assert!(fs_mcp::RiskSource::tool_release(&m, &Target::item("g", "A", "b")).is_none());
    for bad in [
        "[[tool_release]]\ntool = \"A.b\"\nreason = \"r\"\npaths = [\"f/X.y\"]\n",
        "[[tool_release]]\ntool = \"A.b\"\nreason = \" \"\nsource = \"s\"\npaths = [\"f/X.y\"]\n",
        "[[tool_release]]\ntool = \"Ab\"\nreason = \"r\"\nsource = \"s\"\npaths = [\"f/X.y\"]\n",
        "[[tool_release]]\ntool = \"A.b\"\nreason = \"r\"\nsource = \"s\"\npaths = [\"f/X.y\"]\nmore = 1\n",
        "[tool_release]\ntool = \"A.b\"\n",
    ] {
        assert!(MarkFiles::from_sources(&[("f", bad)]).is_err(), "{bad}");
    }
}

#[test]
fn the_real_releases_lift_support_report_and_describe_and_none_is_unused() {
    let catalog = real_catalog();
    let a = Surface::new(&catalog).admit(&On, tasks_mcp::TOOLS);
    // all 25 (20 + the five of `Boards`, 0.0.29, which need no release): support-report by its release (`entries`), describe by its release (`choices`; the label comments are no
    // hits since rule R1 of the comment words)
    assert_eq!(a.offered.len(), 25);
    assert_eq!(a.refused.len(), 0);
    // nothing lifted in vain
    assert_eq!(catalog.unused_releases, Vec::<Target>::new());
    assert_eq!(catalog.unused_release_paths, Vec::<(Target, String)>::new());
    assert_eq!(tasks_mcp::tools(&Off).len(), 0);
}

#[test]
fn a_release_that_lifted_a_hit_is_used_even_if_another_hit_still_refuses_the_tool() {
    // first field: a comment word (lifted), second: a secret name (not lifted)
    let mut plain = field("plain", TypeRef::Text);
    plain.doc = "The id is no secret, only a label".into();
    let c = contract(
        vec![interface(
            "Items",
            vec![method(
                "report",
                vec![],
                vec![field("r", support::st("Report"))],
            )],
        )],
        vec![strukt(
            "Report",
            vec![plain, field("apiToken", TypeRef::Text)],
        )],
    );
    let mut m = FakeMarks::default();
    m.put("Items", "report", RiskClass::Read).release_tool(
        "Items",
        "report",
        &["messenger/Report.plain"],
    );
    let cat = run(&c, &m);
    assert!(!has_tool(&cat, "Items", "report"));
    assert_eq!(
        cat.unused_releases.len(),
        0,
        "the release did lift the comment hit"
    );
}

#[test]
fn a_release_of_a_missing_tool_in_the_mark_files_is_a_stale_target() {
    let text =
        "[[tool_release]]\ntool = \"A.b\"\nreason = \"r\"\nsource = \"s\"\npaths = [\"f/X.y\"]\n";
    let m = MarkFiles::from_sources(&[("f", text)]).unwrap();
    assert_eq!(
        fs_mcp::RiskSource::targets(&m),
        [Target::item("f", "A", "b")]
    );
}

#[test]
fn a_member_tool_is_refused_even_when_its_key_is_in_the_catalog() {
    use support::fx::{catalog, marks as fx_marks, spec_of, table};
    let cat = catalog();
    let t = table(fx_marks());
    let mut s = spec_of(&t, "fx/Notes.get");
    assert_eq!(Surface::new(&cat).refusal(&s), None);
    s.member = Some("any");
    assert_eq!(
        Surface::new(&cat).refusal(&s),
        Some(fs_mcp::Refusal::NotInCatalog)
    );
}
