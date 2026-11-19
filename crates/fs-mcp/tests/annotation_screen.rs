//! The mark `$Cli.secret` in the contract is read by the screen of the catalog: a field with an INNOCENT name and comment is
//! still a secret, and neither a release of the tool nor a release of the field lifts it. `$Cli.sensitive` is not a screen
//! (the field is held back at the output per recipient), so it does not keep a tool out. Pattern: none (tests).

mod support;

use fs_mcp::{CapnpcSource, ContractSource, RiskClass, Target, Why};
use support::fx::workdir_for_tests;
use support::{FakeMarks, run};

const SCHEMA: &str = r#"@0xb8a1c2d3e4f50617;
using Cli = import "/capnpc_cli.capnp";
interface Vault {
  put   @0 (label :Text, note :Text $Cli.secret) -> ();
  mint  @1 (label :Text) -> (note :Text $Cli.secret);
  nest  @2 (label :Text) -> (box :Box);
  show  @3 (label :Text) -> (view :Text $Cli.sensitive);
  plain @4 (label :Text) -> (view :Text);
  pick  @5 (choice :Choice) -> ();
}
struct Choice { union { open @0 :Text; locked @1 :Text $Cli.secret; } }
struct Box { label @0 :Text; note @1 :Text $Cli.secret; }
"#;

fn contracts() -> Vec<fs_mcp::Contract> {
    let dir = workdir_for_tests("annot");
    std::fs::write(dir.join("vault.capnp"), SCHEMA).unwrap();
    let c = CapnpcSource::new(&dir)
        .contracts()
        .expect("the fixture compiles");
    let _ = std::fs::remove_dir_all(&dir);
    c
}

fn marks(release: bool) -> FakeMarks {
    let mut m = FakeMarks::default();
    for item in ["put", "mint", "nest", "show", "plain", "pick"] {
        m.put("Vault", item, RiskClass::Read);
    }
    if release {
        for item in ["put", "mint", "nest"] {
            m.release_tool(
                "Vault",
                item,
                &[
                    &format!("vault/Vault.{item}.note"),
                    "vault/Box.note",
                    "vault/Vault.nest.box",
                ],
            );
        }
        m.releases.insert(
            Target::item("vault", "Vault", "put.note"),
            "false alarm".into(),
        );
        m.releases.insert(
            Target::item("vault", "Vault", "mint.note"),
            "false alarm".into(),
        );
        m.releases
            .insert(Target::item("vault", "Box", "note"), "false alarm".into());
    }
    m
}

fn offered(c: &fs_mcp::Catalog, item: &str) -> bool {
    c.tools
        .iter()
        .any(|t| t.source == Target::item("vault", "Vault", item))
}

#[test]
fn the_annotation_keeps_the_tool_out_for_a_parameter_a_result_and_a_nested_field() {
    let c = run(&contracts(), &marks(false));
    for item in ["put", "mint", "nest"] {
        assert!(!offered(&c, item), "{item} is a tool");
        assert!(
            c.left_out
                .iter()
                .any(|l| l.target == Target::item("vault", "Vault", item)
                    && matches!(&l.why, Why::SecretSuspect { path } if path.ends_with("note"))),
            "{item} is not left out as a secret: {:?}",
            c.left_out
        );
    }
}

#[test]
fn no_release_of_the_tool_or_of_the_field_lifts_it() {
    let c = run(&contracts(), &marks(true));
    for item in ["put", "mint", "nest"] {
        assert!(!offered(&c, item), "{item} is a tool with a release");
    }
}

#[test]
fn sensitive_is_no_screen_and_a_field_without_a_mark_is_offered() {
    let c = run(&contracts(), &marks(false));
    assert!(offered(&c, "show"));
    assert!(offered(&c, "plain"));
}

#[test]
fn the_fields_carry_what_the_contract_says() {
    let all = contracts();
    let f = |s: &str, n: &str| {
        all.iter()
            .flat_map(|c| &c.structs)
            .find(|d| d.name == s)
            .and_then(|d| d.fields.iter().find(|f| f.name == n))
            .map(|f| (f.secret, f.sensitive))
    };
    assert_eq!(f("Box", "note"), Some((true, false)));
    assert_eq!(f("Box", "label"), Some((false, false)));
    let m = |n: &str| {
        all.iter()
            .flat_map(|c| &c.interfaces)
            .find(|i| i.name == "Vault")
            .and_then(|i| i.methods.iter().find(|m| m.name == n))
            .map(|m| {
                m.params
                    .iter()
                    .chain(&m.results)
                    .map(|f| (f.name.clone(), f.secret, f.sensitive))
                    .collect::<Vec<_>>()
            })
    };
    assert_eq!(m("put").unwrap()[1], ("note".into(), true, false));
    assert_eq!(m("mint").unwrap()[1], ("note".into(), true, false));
    assert_eq!(m("show").unwrap()[1], ("view".into(), false, true));
}

#[test]
fn a_secret_member_of_a_union_parameter_keeps_its_own_tool_out() {
    let c = run(&contracts(), &marks(false));
    let tool = |m: &str| {
        c.tools
            .iter()
            .any(|t| t.source == Target::item("vault", "Choice", m))
    };
    assert!(tool("open"), "the open member is a tool: {:?}", c.left_out);
    assert!(!tool("locked"), "the secret member is a tool");
}
