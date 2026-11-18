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
    let out = Command::new(env!("CARGO_BIN_EXE_regenerate"))
        .current_dir(&work)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
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
    let _ = std::fs::remove_dir_all(&work);
}
