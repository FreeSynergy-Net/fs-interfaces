//! The committed generated code must equal what the official compiler (`capnpc-embedded`, capnp
//! 1.4.0 as WASM) produces, and what our own compiler (`capnp-compiler`) produces. This keeps the
//! committed files honest and tests our compiler against the official one on the real schemas.
//!
//! The schemas say `import "/types.capnp"`; the import path is `fs_types::SCHEMA_DIR` (ADR 0093) and the
//! generated code points at `::fs_types::types_capnp`. Output goes to the test target
//! (`CARGO_TARGET_TMPDIR`), never to `/tmp` (memory).

use std::collections::HashMap;

#[path = "../../fs-mcp/src/capnpc_source/stage.rs"]
mod stage;
use std::path::{Path, PathBuf};

const SCHEMAS: [&str; 20] = [
    "common",
    "tasks",
    "messenger",
    "hub",
    "people",
    "groups",
    "files",
    "events",
    "places",
    "texts",
    "surveys",
    "history",
    "resources",
    "whiteboard",
    "streaming",
    "store",
    "comments",
    "settings",
    "layouts",
    "bind",
];

/// File id of `types.capnp` in `fs-types`.
const TYPES_FILE_ID: u64 = 0x916f_ad50_f1c1_4440;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn generate(request: &[u8], tag: &str) -> PathBuf {
    let out = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("fs-interfaces-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    capnpc::codegen::CodeGenerationCommand::new()
        .output_directory(&out)
        .crates_provide_map(HashMap::from([(TYPES_FILE_ID, "fs_types".to_owned())]))
        .run(request)
        .unwrap();
    out
}

fn assert_same(dir: &Path, who: &str) {
    let mut differs = Vec::new();
    for s in SCHEMAS {
        let name = format!("{s}_capnp.rs");
        let committed = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join(&name),
        )
        .unwrap();
        let fresh = std::fs::read_to_string(dir.join(&name)).unwrap();
        if committed != fresh {
            differs.push(name);
        }
    }
    let _ = std::fs::remove_dir_all(dir);
    assert!(
        differs.is_empty(),
        "{differs:?} differ from the output of {who}"
    );
}

#[test]
fn official_compiler_output_is_committed() {
    let schema = root().join("schema");
    let mut cmd = capnpc_embedded::CompileCommand::new();
    let types = stage::reachable(Path::new(fs_types::SCHEMA_DIR)).unwrap();
    cmd.import_path(types.path());
    for s in SCHEMAS {
        cmd.file(schema.join(format!("{s}.capnp")));
    }
    let request = cmd.src_prefix(&schema).compile().expect("official compile");
    assert_same(&generate(&request, "official"), "capnpc-embedded");
}

#[test]
fn own_compiler_output_is_committed() {
    let schema = root().join("schema");
    let mut cmd = capnp_compiler::CompileCommand::new();
    cmd.src_prefix(&schema);
    let types = stage::reachable(Path::new(fs_types::SCHEMA_DIR)).unwrap();
    cmd.import_path(types.path());
    for s in SCHEMAS {
        cmd.file(schema.join(format!("{s}.capnp")));
    }
    let request = cmd.run_bytes().expect("own compile");
    assert_same(&generate(&request, "own"), "capnp-compiler");
}
