//! Regenerates `crates/fs-interfaces/src/*_capnp.rs` from `schema/*.capnp` with our own pure-Rust compiler
//! (`capnp-compiler`, ADR 0040). No C++ toolchain needed. Run from the repository root.
//!
//! ```text
//! cargo run -p regenerate
//! ```
//!
//! The basic types come from `fs-types` (ADR 0093): the schemas say `import "/types.capnp"`, the import path is
//! `fs_types::SCHEMA_DIR`, and the generated code points at `::fs_types::types_capnp`.

use std::collections::HashMap;

use capnp_compiler::CompileCommand;

/// File id of `types.capnp` in `fs-types`.
const TYPES_FILE_ID: u64 = 0x916f_ad50_f1c1_4440;

const SCHEMAS: [&str; 17] = [
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
];

fn main() {
    let mut cmd = CompileCommand::new();
    cmd.src_prefix("schema");
    cmd.import_path(fs_types::SCHEMA_DIR);
    for s in SCHEMAS {
        cmd.file(format!("schema/{s}.capnp"));
    }
    let request = cmd.run_bytes().expect("schema compilation failed");
    std::fs::create_dir_all("crates/fs-interfaces/src").expect("create src");
    capnpc::codegen::CodeGenerationCommand::new()
        .output_directory("crates/fs-interfaces/src")
        .crates_provide_map(HashMap::from([(TYPES_FILE_ID, "fs_types".to_owned())]))
        .run(&request[..])
        .expect("code generation failed");
    println!(
        "regenerated {} files in crates/fs-interfaces/src/",
        SCHEMAS.len()
    );
}
