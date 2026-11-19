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

#[path = "../../fs-mcp/src/capnpc_source/stage.rs"]
mod stage;

mod commands;

use capnp_compiler::CompileCommand;

/// File id of `types.capnp` in `fs-types`.
pub(crate) const TYPES_FILE_ID: u64 = 0x916f_ad50_f1c1_4440;

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

fn main() {
    let mut cmd = CompileCommand::new();
    cmd.src_prefix("schema");
    // On Windows the Cargo checkout can be on another drive than the repository; the compiler reaches one drive only.
    let types = stage::reachable(std::path::Path::new(fs_types::SCHEMA_DIR))
        .expect("schema directory of fs-types");
    println!(
        "fs-types schema: {}",
        if types.is_staged() {
            "copy"
        } else {
            "in place"
        }
    );
    cmd.import_path(types.path());
    // the annotations of the generators, imported by the contracts that carry `$Cli.secret`
    let marks = stage::reachable(std::path::Path::new(capnp_compiler::SCHEMA_DIR))
        .expect("annotation directory of capnp-compiler");
    cmd.import_path(marks.path());
    let import_path = types.path().to_path_buf();
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
    // after the types: the command code reads the same schemas, with the marks of fs-mcp
    commands::write(&import_path);
}
