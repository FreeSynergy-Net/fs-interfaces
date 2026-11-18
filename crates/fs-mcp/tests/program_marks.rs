//! The marks of this program as a `MarkSource` of the generators, driven through the real generator of
//! `capnp-compiler` on a small contract, and the screen on top. Pattern: none (tests).

mod support;

use std::path::PathBuf;

use capnp_compiler::CompileCommand;
use capnp_compiler::cligen::MarkSource;
use capnp_compiler::mcpgen::GenerationCommand;
use capnp_compiler::mcpgen::rt::{Danger, ToolSpec};
use fs_mcp::{Blocklist, MarkFiles, ProgramMarks, Refusal, RiskSource, Surface, Target};
use support::{FakeMarks, contract, field, interface, method, strukt};

/// `secret` is declared here with the id of `capnpc_cli.capnp`: the generator finds the annotation by id.
const SCHEMA: &str = r"@0xd1a2b3c4d5e6f701;
annotation secret @0xefac3d5046acfeb0 (field, param) :Void;
struct Note { id @0 :UInt32; title @1 :Text; }
interface Root { notes @0 () -> (notes :Notes); }
interface Notes {
  get     @0 (id :UInt32) -> (note :Note);
  login   @1 (password :Text) -> ();
  issue   @2 () -> (value :Text $secret);
  peek    @3 () -> (apiToken :Text);
  vault   @4 () -> ();
  drop    @5 (id :UInt32) -> ();
  named   @6 () -> ();
  loginOk @7 (password :Text) -> ();
  nested  @8 (box :Note, otherKey :Text) -> ();
}
";

const MARKS: &str = r#"
[Notes]
get = "read"
login = "add"
issue = "read"
peek = "read"
vault = { risk = "excluded", reason = "admin" }
drop = "destructive"
named = { risk = "acts-in-name", capability = "note.named" }
loginOk = "add"
nested = "change"
[Root]
notes = "read"
[release]
"Notes.loginOk.password" = "a password hint field the person types, released in writing"
"#;

/// A fresh folder per call: the tests of one process run in parallel, and the processes of several runs too.
fn workdir(tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("pm-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn marks() -> ProgramMarks<MarkFiles> {
    ProgramMarks::new(MarkFiles::from_sources(&[("fx", MARKS)]).unwrap())
}

fn table(marks: ProgramMarks<MarkFiles>) -> String {
    let dir = workdir("gen");
    std::fs::write(dir.join("fx.capnp"), SCHEMA).unwrap();
    let mut cmd = CompileCommand::new();
    cmd.src_prefix(&dir);
    cmd.file(dir.join("fx.capnp"));
    let request = cmd.run_bytes().expect("fixture compiles");
    let mut g = GenerationCommand::new();
    g.marks(Box::new(marks));
    let files = g.generate(&request).expect("tables generate");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(files.len(), 1);
    files.into_iter().next().unwrap().content
}

fn has_tool(text: &str, name: &str) -> bool {
    text.contains(&format!("name: \"{name}\","))
}

fn left(text: &str, key: &str) -> Option<String> {
    let line = text
        .lines()
        .find(|l| l.trim_start().starts_with(&format!("(\"{key}\", ")))?;
    Some(
        line.rsplit_once(", \"")?
            .1
            .trim_end_matches("\"),")
            .to_owned(),
    )
}

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

fn table_without_marks() -> String {
    let dir = workdir("bare");
    std::fs::write(dir.join("fx.capnp"), SCHEMA).unwrap();
    let mut cmd = CompileCommand::new();
    cmd.src_prefix(&dir);
    cmd.file(dir.join("fx.capnp"));
    let request = cmd.run_bytes().unwrap();
    let files = GenerationCommand::new().generate(&request).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    files.into_iter().next().unwrap().content
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

/// The tool of the table for `key`, as data (`ToolSpec` has only `'static` text; the text of a generated file is read).
fn spec_of(text: &str, key: &str) -> ToolSpec {
    let at = text
        .find(&format!("key: \"{key}\","))
        .expect("key in table");
    let meta = {
        let rest = &text[at..];
        let from = rest.find("meta: r#\"").unwrap() + "meta: r#\"".len();
        let len = rest[from..].find("\"#").unwrap();
        rest[from..from + len].to_owned()
    };
    ToolSpec {
        name: "fx.notes.peek",
        title: "peek",
        description: "d",
        key: Box::leak(key.to_owned().into_boxed_str()),
        member: None,
        words: &["notes", "peek"],
        input_schema: "{}",
        output_schema: "{}",
        meta: Box::leak(meta.into_boxed_str()),
        risk: Some(Danger::Read),
        free_content: false,
    }
}

/// The catalog of the same contract and marks, made by this crate (its own reader of the contract).
fn catalog() -> fs_mcp::Catalog {
    use fs_mcp::{ContractSource, Generator, ToolGenerator};
    let dir = workdir("cat");
    std::fs::write(dir.join("fx.capnp"), SCHEMA.replace("$secret", "")).unwrap();
    let contracts = fs_mcp::CapnpcSource::new(&dir).contracts().unwrap();
    let marks = MarkFiles::from_sources(&[("fx", MARKS)]).unwrap();
    let c = Generator::new(&marks, Blocklist::standard())
        .generate(&contracts)
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    c
}

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
fn a_union_member_is_matched_at_the_union_in_its_file() {
    use support::{FILE, has_tool, member, run, st};
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
    assert!(
        has_tool(&cat, "Act", "flag") && has_tool(&cat, "Act", "mute"),
        "both members are tools (mute is unmarked)"
    );
    let t = table(self::marks());
    let mut spec = spec_of(&t, "fx/Notes.get");
    spec.key = "messenger/Conversation.act";
    spec.member = Some("flag");
    let surface = Surface::new(&cat);
    assert_eq!(surface.refusal(&spec), None);
    spec.member = Some("nothing");
    assert_eq!(surface.refusal(&spec), Some(Refusal::NotInCatalog));
    spec.member = Some("flag");
    spec.key = "other/Conversation.act";
    assert_eq!(
        surface.refusal(&spec),
        Some(Refusal::NotInCatalog),
        "another schema file is another contract"
    );
    let _ = (FILE, RiskSource::targets(&FakeMarks::default()));
}
