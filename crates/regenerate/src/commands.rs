//! The command code and the tool tables of `tasks.capnp` (`tasks_cli.rs`, `tasks_mcp.rs`), written into `fs-mcp`.
//!
//! They live in `fs-mcp`, not in `fs-interfaces`: the contract crate stays free of the run time of the generators
//! (`capnp-compiler`, `capnp-rpc`), and the program that serves the tools is the one that holds the code. The code
//! names the contracts of `fs-interfaces` and the basic types of `fs-types` by crate (`crates_provide_map`).
//!
//! Pattern: Facade — `write` hides the compilation and the two generators behind one call.

use std::collections::HashMap;
use std::path::Path;

use capnp_compiler::CompileCommand;
use capnp_compiler::{cligen, mcpgen};
use fs_mcp::{MarkFiles, ProgramMarks};

/// File ids (`@0x...` at the top of the schema files).
const TASKS_FILE_ID: u64 = 0xc04d_2589_7154_3476;
const COMMON_FILE_ID: u64 = 0xb3f1_c2d4_e5a6_9788;
const TYPES_FILE_ID: u64 = super::TYPES_FILE_ID;

/// Where the files go, relative to the repository root.
pub const OUT_DIR: &str = "crates/fs-mcp/src/generated";

fn marks() -> ProgramMarks<MarkFiles> {
    // The mark files of `fs-mcp`, found from the source of this tool (it is run from a checkout of this repository).
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fs-mcp/marks");
    ProgramMarks::new(MarkFiles::from_dir(&dir).expect("the mark files are valid"))
}

/// Compile `schema/tasks.capnp` (the only file with a command line and tools so far) and write the two files.
pub fn write(import_path: &Path) {
    let mut cmd = CompileCommand::new();
    cmd.src_prefix("schema");
    cmd.import_path(import_path);
    cmd.file("schema/tasks.capnp");
    let request = cmd.run_bytes().expect("tasks.capnp compiles");
    std::fs::create_dir_all(OUT_DIR).expect("create the output directory");
    let mut cli = cligen::GenerationCommand::new();
    cli.crates_provide_map(HashMap::from([
        (TASKS_FILE_ID, "fs_interfaces".to_owned()),
        (COMMON_FILE_ID, "fs_interfaces".to_owned()),
        (TYPES_FILE_ID, "fs_types".to_owned()),
    ]))
    .marks(Box::new(marks()))
    .output_directory(OUT_DIR)
    .run(&request[..])
    .expect("command code generation failed");
    let mut mcp = mcpgen::GenerationCommand::new();
    mcp.marks(Box::new(marks()))
        .output_directory(OUT_DIR)
        .run(&request[..])
        .expect("tool table generation failed");
    println!("wrote tasks_cli.rs and tasks_mcp.rs into {OUT_DIR}/");
}
