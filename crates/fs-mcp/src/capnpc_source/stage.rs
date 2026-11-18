//! Schema directories on another drive (Windows).
//!
//! Both schema compilers (`capnp-compiler` and the embedded official one) reach only the drive of the current
//! directory: they drop the drive letter of a path and resolve the rest on the current drive. A directory such as
//! `fs_types::SCHEMA_DIR` (inside the Cargo checkout, `C:\Users\..\.cargo`) is therefore "no such file" when the work
//! happens on `D:`. [`reachable`] copies the `.capnp` files of such a directory into a folder below the current
//! directory and removes it again when the guard is dropped. On the same drive (and everywhere except Windows) nothing is
//! copied and the directory is used as it is.
//!
//! Pattern: Guard (RAII), the copy lives as long as the value and is removed on drop.
//!
//! The file depends on `std` only: `crates/regenerate` and the tests of `fs-interfaces` include it with `#[path]`.
#![allow(dead_code)]

use std::path::{Component, Path, PathBuf, Prefix};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Makes the staging folders of one process unique (tests run in parallel).
static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A directory the compilers can reach; a staged copy is removed on drop.
pub struct Reachable {
    path: PathBuf,
    staged: bool,
}

impl Reachable {
    /// The directory to hand to the compiler as import path (and as source prefix, to keep the file names).
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the directory is a copy.
    #[must_use]
    pub fn is_staged(&self) -> bool {
        self.staged
    }
}

impl Drop for Reachable {
    fn drop(&mut self) {
        if self.staged {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

/// The upper-case drive letter of a path (`C:\x`, `\\?\C:\x`); `None` for every path without one (all of Unix).
#[must_use]
pub fn drive_of(p: &Path) -> Option<u8> {
    match p.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(d) | Prefix::VerbatimDisk(d) => Some(d.to_ascii_uppercase()),
            _ => None,
        },
        _ => None,
    }
}

/// Whether the compilers (working from `cwd`) can reach `dir` without a copy.
#[must_use]
pub fn same_drive(dir: Option<u8>, cwd: Option<u8>) -> bool {
    dir == cwd || dir.is_none()
}

/// `dir` itself when it is reachable from the current directory, else a copy of its `.capnp` files below it.
///
/// # Errors
/// A message with the path when `dir` is no directory or a file cannot be copied.
pub fn reachable(dir: &Path) -> Result<Reachable, String> {
    if !dir.is_dir() {
        return Err(format!("schema directory {} does not exist", dir.display()));
    }
    let cwd = std::env::current_dir().map_err(|e| format!("current directory: {e}"))?;
    if same_drive(drive_of(dir), drive_of(&cwd)) {
        return Ok(Reachable {
            path: dir.to_path_buf(),
            staged: false,
        });
    }
    stage(
        dir,
        &cwd.join(format!(
            "fs-schema-stage-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        )),
    )
}

/// Copy the `.capnp` files of `dir` into `into` (a new folder).
///
/// # Errors
/// A message with the path of the failing step; `into` must not exist yet (it is created, and removed again on error).
pub fn stage(dir: &Path, into: &Path) -> Result<Reachable, String> {
    let fail = |what: &str, p: &Path, e: std::io::Error| format!("{what} {}: {e}", p.display());
    // `create_dir`, not `create_dir_all`: an existing folder must fail (the guard would remove somebody else's content).
    std::fs::create_dir(into).map_err(|e| fail("cannot create", into, e))?;
    let guard = Reachable {
        path: into.to_path_buf(),
        staged: true,
    };
    for entry in std::fs::read_dir(dir).map_err(|e| fail("cannot read", dir, e))? {
        let from = entry.map_err(|e| fail("cannot read", dir, e))?.path();
        if from.extension().is_some_and(|x| x == "capnp") {
            let to = into.join(from.file_name().unwrap_or_default());
            std::fs::copy(&from, &to).map_err(|e| fail("cannot copy", &from, e))?;
        }
    }
    Ok(guard)
}
