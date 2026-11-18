//! The marks of this program as a `MarkSource` of the generators, driven through the real generator of
//! `capnp-compiler` on a small contract. Pattern: none (tests).

mod support;

use capnp_compiler::cligen::MarkSource;
use fs_mcp::{Blocklist, MarkFiles, ProgramMarks};
use support::FakeMarks;
use support::fx::{has_tool, left, marks, table, table_without_marks};

#[test]
fn the_marks_decide_what_becomes_a_tool_and_how_it_is_named() {
    let t = table(marks());
    assert!(has_tool(&t, "fx.notes.get"));
    assert!(has_tool(&t, "fx.notes.drop"));
    // a capability id is the name of the tool
    assert!(has_tool(&t, "note.named") && !has_tool(&t, "fx.notes.named"));
    // marked excluded: no tool, and the table says why it is left out
    assert!(!has_tool(&t, "fx.notes.vault"));
    assert_eq!(left(&t, "fx/Notes.vault").as_deref(), Some("excluded"));
    // risk comes from the marks
    assert!(
        t.contains("risk: Some(Danger::Destructive)")
            && t.contains("risk: Some(Danger::ActsInName)")
    );
    assert!(t.contains("risk: Some(Danger::Read)") && t.contains("risk: Some(Danger::Add)"));
}

#[test]
fn a_secret_looking_parameter_is_no_tool_unless_released_in_writing() {
    let t = table(marks());
    assert!(!has_tool(&t, "fx.notes.login"));
    assert_eq!(left(&t, "fx/Notes.login").as_deref(), Some("secret"));
    // the written release for exactly this parameter lets it through
    assert!(has_tool(&t, "fx.notes.login-ok"));
    // the same without any program marks: the schema says nothing, so the generator alone lets it through
    let bare = table_without_marks();
    assert!(has_tool(&bare, "fx.notes.login"));
}

#[test]
fn a_secret_annotation_on_a_result_field_is_no_tool() {
    // `issue` gives `value :Text $secret`: the annotation counts for results, with or without program marks
    for t in [table(marks()), table_without_marks()] {
        assert!(!has_tool(&t, "fx.notes.issue"));
        assert_eq!(left(&t, "fx/Notes.issue").as_deref(), Some("secret"));
    }
}

#[test]
fn a_name_in_a_nested_path_and_a_kebab_name_are_read_in_the_contract_spelling() {
    let m = marks();
    // `other-key` is `otherKey` in the contract: a secret word
    assert!(m.secret("fx/Notes.nested", "other-key"));
    assert!(m.secret("fx/Notes.nested", "box.other-key"));
    assert!(m.secret("fx/Notes.nested", "box.*.other-key"));
    assert!(!m.secret("fx/Notes.nested", "box"));
    assert!(!m.secret("fx/Notes.get", "id"));
    // released: this very parameter, or a field of that name in the same file
    assert!(!m.secret("fx/Notes.loginOk", "password"));
    assert!(m.secret("fx/Notes.login", "password"));
    assert!(
        m.secret("other/Notes.loginOk", "password"),
        "a release in another file does not count"
    );
}

#[test]
fn the_block_list_and_interface_exclusions_apply_to_the_keys_of_the_generators() {
    let m = marks();
    assert!(m.excluded("fx/Notes.vault"));
    assert!(!m.excluded("fx/Notes.get"));
    // the interface as a whole, and a navigation method to it
    let all = ProgramMarks::new(
        MarkFiles::from_sources(&[("fx", "[Notes]\n\"@interface\" = { risk = \"excluded\", reason = \"admin\" }\nget = \"read\"\n[Root]\nnotes = \"read\"\n")]).unwrap(),
    );
    assert!(all.excluded("fx/Notes.get") && all.excluded("fx/Notes"));
    assert!(!all.excluded("fx/Root.notes"));
    // a resource is never a tool for now
    let res = ProgramMarks::new(
        MarkFiles::from_sources(&[(
            "fx",
            "[Notes]\nget = { risk = \"read\", resource = true }\n",
        )])
        .unwrap(),
    );
    assert!(res.excluded("fx/Notes.get"));
    // the block list, by target and by type name
    let block = ProgramMarks::new(FakeMarks::default());
    assert!(block.excluded("messenger/MessageAction.decide"));
    assert!(block.excluded("messenger/Guard.allow"));
    assert!(!block.excluded("messenger/Messenger.capabilities"));
    let open = ProgramMarks::with_blocklist(FakeMarks::default(), Blocklist::empty());
    assert!(!open.excluded("messenger/MessageAction.decide"));
    // a key that is no key
    assert!(
        !m.excluded("nonsense")
            && !m.secret("nonsense", "x")
            && m.risk("nonsense").is_none()
            && m.tool_name("nonsense").is_none()
    );
    assert_eq!(m.tool_name("fx/Notes.named").as_deref(), Some("note.named"));
    assert_eq!(m.tool_name("fx/Notes.get"), None);
}

fn with(text: &str) -> ProgramMarks<MarkFiles> {
    ProgramMarks::new(MarkFiles::from_sources(&[("fx", text)]).unwrap())
}

#[test]
fn a_released_name_is_matched_in_the_spelling_of_the_contract() {
    // the generators say `other-key`, the release says `otherKey`
    let m = with(
        "[Notes]\nnested = \"change\"\n[release]\n\"Notes.nested.otherKey\" = \"a label, not a key\"\n",
    );
    assert!(!m.secret("fx/Notes.nested", "other-key"));
    assert!(!m.secret("fx/Notes.nested", "box.other-key"));
    assert!(
        m.secret("fx/Notes.other", "other-key"),
        "the release is for one method"
    );
}

#[test]
fn a_field_release_counts_in_its_own_file_and_only_as_a_release() {
    // a release of a field of a struct (`Reaction.password`) lets a parameter of that name through, in that file only
    let m = with(
        "[Reaction]\npassword = \"read\"\n[release]\n\"Hint.password\" = \"a hint text, not a password\"\n",
    );
    assert!(!m.secret("fx/Notes.login", "password"));
    assert!(
        m.secret("other/Notes.login", "password"),
        "a release of another file does not count"
    );
    // a mark (no release) on a field of that name is no release
    let m = with("[Reaction]\npassword = \"read\"\n");
    assert!(
        m.secret("fx/Notes.login", "password"),
        "a mark is not a release"
    );
}

#[test]
fn a_type_on_the_block_list_blocks_every_command_of_its_interface() {
    // `AiGrant` is blocked as a type only (no entry for a target), in any file
    let m = ProgramMarks::new(FakeMarks::default());
    assert!(m.excluded("any/AiGrant.grant"));
    assert!(!m.excluded("any/Grants.grant"));
}
