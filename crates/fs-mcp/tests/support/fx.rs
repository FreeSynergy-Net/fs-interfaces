//! The fixture contract of the marks tests: a schema, its marks, and the tables the generator writes for it.
//! Test helpers; no pattern of their own.
#![allow(dead_code)]

use std::path::PathBuf;

use capnp_compiler::CompileCommand;
use capnp_compiler::mcpgen::GenerationCommand;
use capnp_compiler::mcpgen::rt::{Danger, ToolSpec};
use fs_mcp::{Blocklist, MarkFiles, ProgramMarks};

/// `secret` is declared here with the id of `capnpc_cli.capnp`: the generator finds the annotation by id.
pub const SCHEMA: &str = r"@0xd1a2b3c4d5e6f701;
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

pub const MARKS: &str = r#"
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
pub fn workdir(tag: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("pm-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

pub fn marks() -> ProgramMarks<MarkFiles> {
    ProgramMarks::new(MarkFiles::from_sources(&[("fx", MARKS)]).unwrap())
}

pub fn table(marks: ProgramMarks<MarkFiles>) -> String {
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

pub fn has_tool(text: &str, name: &str) -> bool {
    text.contains(&format!("name: \"{name}\","))
}

pub fn left(text: &str, key: &str) -> Option<String> {
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
pub fn table_without_marks() -> String {
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
/// The tool of the table for `key`, as data (`ToolSpec` has only `'static` text; the text of a generated file is read).
pub fn spec_of(text: &str, key: &str) -> ToolSpec {
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
pub fn catalog() -> fs_mcp::Catalog {
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
