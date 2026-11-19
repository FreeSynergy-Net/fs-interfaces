//! The screen (`Surface`) on a small contract: a result that looks like a secret, a missing AI label, an excluded tool, and
//! a union member. Pattern: none (tests).

mod support;

use capnp_compiler::mcpgen::rt::Danger;
use fs_mcp::{Refusal, Surface, Target};
use support::fx::{catalog, has_tool, marks, spec_of, table};
use support::{FakeMarks, contract, field, interface, method, strukt};

#[test]
fn a_result_field_that_looks_like_a_secret_is_cut_by_the_screen_not_by_the_generator() {
    let t = table(marks());
    // The generator reads no marks for results: `peek` gives `apiToken` and is in the table ...
    assert!(has_tool(&t, "fx.notes.peek"));
    // ... the catalog of this crate keeps it out, so the intersection does too: no tool, no call, no result.
    let cat = catalog();
    assert!(
        cat.left_out
            .iter()
            .any(|l| l.target == Target::item("fx", "Notes", "peek")),
        "{:?}",
        cat.left_out
    );
    let peek = spec_of(&t, "fx/Notes.peek");
    let get = spec_of(&t, "fx/Notes.get");
    let surface = Surface::new(&cat);
    assert_eq!(surface.refusal(&peek), Some(Refusal::NotInCatalog));
    assert_eq!(surface.refusal(&get), None);
}

#[test]
fn the_screen_refuses_a_tool_without_the_ai_label_and_an_excluded_one() {
    let cat = catalog();
    let t = table(marks());
    let surface = Surface::new(&cat);
    let mut human = spec_of(&t, "fx/Notes.get");
    human.meta = r#"{"net.freesynergy/mcp":{"actor":"human"}}"#;
    assert_eq!(surface.refusal(&human), Some(Refusal::NoAiLabel));
    let mut broken = spec_of(&t, "fx/Notes.get");
    broken.meta = "";
    assert_eq!(surface.refusal(&broken), Some(Refusal::NoAiLabel));
    let mut ex = spec_of(&t, "fx/Notes.get");
    ex.risk = Some(Danger::Excluded);
    assert_eq!(surface.refusal(&ex), Some(Refusal::Excluded));
    // excluded wins over everything else, also over a missing label
    ex.meta = "";
    assert_eq!(surface.refusal(&ex), Some(Refusal::Excluded));
}

#[test]
fn a_union_member_tool_is_refused_until_both_sides_split_unions_alike() {
    use support::{has_tool, member, run, st};
    let c = contract(
        vec![interface(
            "Conversation",
            vec![method("act", vec![field("action", st("Act"))], vec![])],
        )],
        vec![strukt(
            "Act",
            vec![
                member("flag", fs_mcp::TypeRef::Bool),
                member("mute", fs_mcp::TypeRef::Void),
            ],
        )],
    );
    let mut marks = FakeMarks::default();
    marks.put("Act", "flag", fs_mcp::RiskClass::Change);
    let cat = run(&c, &marks);
    assert!(has_tool(&cat, "Act", "flag"));
    let t = table(self::marks());
    let mut spec = spec_of(&t, "fx/Notes.get");
    spec.key = "messenger/Conversation.act";
    let surface = Surface::new(&cat);
    for m in [Some("flag"), Some("nothing"), Some("Act.flag")] {
        spec.member = m;
        assert_eq!(surface.refusal(&spec), Some(Refusal::NotInCatalog), "{m:?}");
    }
}

#[test]
fn a_method_is_matched_by_its_whole_key_not_by_a_prefix() {
    use support::{has_tool, run};
    let c = contract(
        vec![interface(
            "Items",
            vec![
                method("get", vec![], vec![]),
                method("getByUid", vec![], vec![]),
            ],
        )],
        vec![],
    );
    let mut marks = FakeMarks::default();
    marks.put("Items", "getByUid", fs_mcp::RiskClass::Read);
    let mut cat = run(&c, &marks);
    cat.tools
        .retain(|t| t.source.as_str() == "messenger/Items.getByUid");
    assert_eq!(cat.tools.len(), 1);
    assert!(!has_tool(&cat, "Items", "get"));
    let t = table(self::marks());
    let mut spec = spec_of(&t, "fx/Notes.get");
    spec.key = "messenger/Items.get";
    let surface = Surface::new(&cat);
    assert_eq!(surface.refusal(&spec), Some(Refusal::NotInCatalog));
    spec.key = "messenger/Items.getByUid";
    assert_eq!(surface.refusal(&spec), None);
    spec.key = "messenger/Items.getBy";
    assert_eq!(surface.refusal(&spec), Some(Refusal::NotInCatalog));
}

#[test]
fn only_the_exact_value_ai_is_the_label_and_the_label_is_checked_before_the_catalog() {
    let cat = catalog();
    let t = table(marks());
    let surface = Surface::new(&cat);
    for actor in ["aix", "ai:x", "AI", " ai", "", "human"] {
        let mut s = spec_of(&t, "fx/Notes.get");
        s.meta = Box::leak(
            format!("{{\"net.freesynergy/mcp\":{{\"actor\":\"{actor}\"}}}}").into_boxed_str(),
        );
        assert_eq!(surface.refusal(&s), Some(Refusal::NoAiLabel), "{actor:?}");
    }
    // no label and not in the catalog: the label is the first reason
    let mut s = spec_of(&t, "fx/Notes.peek");
    s.meta = "{}";
    assert_eq!(surface.refusal(&s), Some(Refusal::NoAiLabel));
}
