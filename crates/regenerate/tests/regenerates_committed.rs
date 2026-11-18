//! The `regenerate` binary, run on a copy of `schema/`, writes exactly the committed generated files (one per schema, none
//! missing, none more): a schema left out of its list, or a broken run, is red here. The run happens in a folder of the
//! test target, never in the repository.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn names(dir: &Path, ext: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(ext))
        .collect();
    v.sort();
    v
}

#[test]
fn the_binary_writes_exactly_the_committed_generated_files() {
    let work =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("regenerate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(work.join("schema")).unwrap();
    for name in names(&root().join("schema"), ".capnp") {
        std::fs::copy(
            root().join("schema").join(&name),
            work.join("schema").join(&name),
        )
        .unwrap();
    }
    // The copy of the schema directory of fs-types (the Windows path) is forced, so Linux walks it too.
    let out = Command::new(env!("CARGO_BIN_EXE_regenerate"))
        .current_dir(&work)
        .env("FS_SCHEMA_STAGE_ALWAYS", "1")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("fs-types schema: copy"),
        "regenerate reads the schema directory of fs-types through the staging guard"
    );
    let fresh = work.join("crates/fs-interfaces/src");
    let committed = root().join("crates/fs-interfaces/src");
    let fresh_names = names(&fresh, "_capnp.rs");
    // one generated file per schema file
    let expected: Vec<String> = names(&root().join("schema"), ".capnp")
        .iter()
        .map(|n| format!("{}_capnp.rs", n.trim_end_matches(".capnp")))
        .collect();
    let mut expected = expected;
    expected.sort();
    assert_eq!(
        fresh_names, expected,
        "regenerate writes one file per schema"
    );
    for name in &fresh_names {
        assert_eq!(
            std::fs::read_to_string(fresh.join(name)).unwrap(),
            std::fs::read_to_string(committed.join(name)).unwrap(),
            "{name} differs from the committed file"
        );
    }
    // the command code and the tool table of tasks.capnp, written into fs-mcp with its marks
    let fresh = work.join("crates/fs-mcp/src/generated");
    let committed = root().join("crates/fs-mcp/src/generated");
    assert_eq!(
        names(&fresh, ".rs"),
        ["tasks_cli.rs", "tasks_mcp.rs"],
        "regenerate writes the command code and the tool table of tasks.capnp, nothing else"
    );
    assert_eq!(
        names(&committed, ".rs"),
        names(&fresh, ".rs"),
        "no committed generated file that regenerate does not write"
    );
    for name in names(&fresh, ".rs") {
        assert_eq!(
            std::fs::read_to_string(fresh.join(&name)).unwrap(),
            std::fs::read_to_string(committed.join(&name)).unwrap(),
            "{name} differs from the committed file (run `cargo run -p regenerate`)"
        );
    }
    let _ = std::fs::remove_dir_all(&work);
}
