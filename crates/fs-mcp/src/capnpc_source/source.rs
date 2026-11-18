//! The [`ContractSource`] over `capnp-compiler`.
//!
//! Pattern: Adapter — wraps `capnp-compiler` and `capnp::schema_capnp` behind [`ContractSource`].
//! Trait: [`ContractSource`].

use std::path::{Path, PathBuf};

use capnp::schema_capnp::code_generator_request;
use capnp_compiler::CompileCommand;

use crate::{Contract, ContractSource, Error};

/// All `*.capnp` files of one directory, compiled together.
pub struct CapnpcSource {
    dir: PathBuf,
    /// Directories of other crates' schemas: import path, and their contracts are listed too.
    extra: Vec<PathBuf>,
}

impl CapnpcSource {
    /// Contracts in `dir` (for example the `schema` directory of `fs-interfaces`).
    #[must_use]
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
            extra: Vec::new(),
        }
    }

    /// Also reads the contracts in `dir` (for example `fs_types::SCHEMA_DIR`, ADR 0093): it becomes an import
    /// path, so `import "/types.capnp"` resolves, and its contracts are listed like the others.
    #[must_use]
    pub fn with_schemas(mut self, dir: impl AsRef<Path>) -> Self {
        self.extra.push(dir.as_ref().to_path_buf());
        self
    }
}

fn capnp_files(dir: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| Error::Compile(format!("{}: {e}", dir.display())))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "capnp"))
        .collect();
    files.sort();
    Ok(files)
}

impl ContractSource for CapnpcSource {
    fn contracts(&self) -> Result<Vec<Contract>, Error> {
        let main = super::stage::reachable(&self.dir).map_err(Error::Compile)?;
        let mut files = capnp_files(main.path())?;
        let mut cmd = CompileCommand::new();
        cmd.src_prefix(main.path());
        // Kept alive until the compilation is done: a staged copy (another drive on Windows) is removed on drop.
        let mut staged = Vec::new();
        for extra in &self.extra {
            let dir = super::stage::reachable(extra).map_err(Error::Compile)?;
            cmd.import_path(dir.path());
            if dir.is_staged() {
                cmd.src_prefix(dir.path());
            }
            files.extend(capnp_files(dir.path())?);
            staged.push(dir);
        }
        for f in &files {
            cmd.file(f);
        }
        let bytes = cmd
            .run_bytes()
            .map_err(|e| Error::Compile(format!("{e:?}")))?;
        let message = capnp::serialize::read_message(
            &mut &bytes[..],
            capnp::message::ReaderOptions {
                traversal_limit_in_words: None,
                nesting_limit: 128,
            },
        )?;
        let request: code_generator_request::Reader<'_> = message.get_root()?;
        super::nodes::read_request(request)
    }
}
