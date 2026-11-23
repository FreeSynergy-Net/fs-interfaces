//! The generator must build and run when its own output is missing or stale (it would not compile against a new
//! `fs-types` before it ran): `regenerate` takes `fs-mcp` without the feature `generated`, so `tasks_cli.rs` and
//! `tasks_mcp.rs` are not part of what it builds. Pattern: none (tests).

use std::path::Path;
use std::process::Command;

#[test]
fn regenerate_does_not_build_the_generated_files_of_fs_mcp() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new(env!("CARGO"))
        .args([
            "tree",
            "-p",
            "regenerate",
            "-e",
            "features",
            "--locked",
            "--offline",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    let tree = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{tree}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        tree.contains("fs-mcp"),
        "regenerate still reads the marks of fs-mcp: {tree}"
    );
    assert!(
        !tree.contains("fs-mcp feature \"generated\""),
        "regenerate must not switch on the feature `generated` of fs-mcp (it would build the stale output first):\n{tree}"
    );
}
