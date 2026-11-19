//! The marks `$Cli.secret` in the REAL contracts: exactly these fields carry it, and the tools of the generator leave out every command
//! that takes or gives one (a secret is no tool, not even with a release). Pattern: none (tests).

#[path = "../src/capnpc_source/stage.rs"]
mod stage;

use std::path::{Path, PathBuf};

use capnp_compiler::cligen::model::{Model, SECRET_ANNOTATION};
use capnp_compiler::{CompileCommand, SCHEMA_DIR};

fn schema_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schema")
}

fn request(files: &[&str]) -> Vec<u8> {
    let dir = schema_dir();
    let types = stage::reachable(Path::new(fs_types::SCHEMA_DIR)).unwrap();
    let marks = stage::reachable(Path::new(SCHEMA_DIR)).unwrap();
    let mut cmd = CompileCommand::new();
    cmd.src_prefix(&dir);
    cmd.import_path(types.path());
    cmd.import_path(marks.path());
    for f in files {
        cmd.file(dir.join(format!("{f}.capnp")));
    }
    cmd.run_bytes().expect("the real contracts compile")
}

fn all_schemas() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(schema_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "capnp"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

/// Every field of the real contracts that carries `$Cli.secret`, as `Struct.field`.
fn secret_fields() -> Vec<String> {
    let files = all_schemas();
    let refs: Vec<&str> = files.iter().map(String::as_str).collect();
    let model = Model::from_request(&request(&refs)).expect("reads");
    let mut out: Vec<String> = model
        .structs
        .values()
        .flat_map(|s| {
            s.fields
                .iter()
                .filter(|f| f.annotations.iter().any(|a| a.id == SECRET_ANNOTATION))
                .map(|f| {
                    format!(
                        "{}.{}",
                        s.name.rsplit(':').next().unwrap_or(&s.name),
                        f.name
                    )
                })
        })
        .collect();
    out.sort();
    out
}

#[test]
fn exactly_these_fields_of_the_real_contracts_are_secrets() {
    assert_eq!(
        secret_fields(),
        [
            "AccountSetup.secret",
            "Credential.token",
            "Streams.addTarget$Params.key",
            "Streams.create$Results.publisherKey",
            "Streams.rotateKey$Results.publisherKey",
            "Streams.setTargetKey$Params.key",
        ],
        "a new or a lost mark of a secret must be a decision that a human reads (see the review of the contract)"
    );
}

/// The text of the tool table of one contract, and the keys it leaves out as secrets.
fn left_as_secret(file: &str) -> (String, Vec<String>) {
    let text: String = capnp_compiler::mcpgen::GenerationCommand::new()
        .generate(&request(&[file]))
        .expect("generates")
        .into_iter()
        .map(|f| f.content)
        .collect();
    let mut keys: Vec<String> = text
        .lines()
        .filter_map(|l| l.trim().strip_prefix("(\""))
        .filter_map(|l| l.split_once("\", \"secret\")"))
        .map(|(k, _)| k.to_owned())
        .collect();
    keys.sort();
    (text, keys)
}

#[test]
fn a_command_with_a_secret_is_no_tool_in_the_real_contracts() {
    for (file, keys) in [
        ("messenger", vec!["messenger/Accounts.add"]),
        ("surveys", vec!["surveys/Respond.submit"]),
        (
            "streaming",
            vec![
                "streaming/Streams.addTarget",
                "streaming/Streams.create",
                "streaming/Streams.rotateKey",
                "streaming/Streams.setTargetKey",
            ],
        ),
    ] {
        let (text, left) = left_as_secret(file);
        assert_eq!(left, keys, "{file}");
        for k in keys {
            assert!(!text.contains(&format!("key: \"{k}\"")), "{k} is a tool");
        }
    }
}
