//! Schema directories on another drive (Windows): the compilers reach only the drive of the current directory, so such a
//! directory is copied below the current directory. These tests run on every system; the drive parts only on Windows.

use std::path::Path;

#[path = "../../fs-mcp/src/capnpc_source/stage.rs"]
mod stage;

fn scratch(name: &str) -> std::path::PathBuf {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("stage-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_missing_directory_is_named_in_the_message() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-such-schema-dir");
    let Err(message) = stage::reachable(&dir) else {
        panic!("a missing directory is an error")
    };
    assert!(message.contains("no-such-schema-dir"), "{message}");
}

#[test]
fn a_directory_on_the_same_drive_is_used_as_it_is() {
    let dir = scratch("same");
    let r = stage::reachable(&dir).unwrap();
    assert!(!r.is_staged());
    assert_eq!(r.path(), dir.as_path());
    drop(r);
    assert!(
        dir.is_dir(),
        "a directory that was not staged is never removed"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn staging_copies_only_schemas_and_removes_the_copy_on_drop() {
    let from = scratch("from");
    std::fs::write(from.join("a.capnp"), "@0x1;").unwrap();
    std::fs::write(from.join("b.txt"), "x").unwrap();
    let into = from.join("copy");
    let staged = stage::stage(&from, &into).unwrap();
    assert!(staged.is_staged());
    assert_eq!(
        std::fs::read_to_string(into.join("a.capnp")).unwrap(),
        "@0x1;"
    );
    assert!(!into.join("b.txt").exists());
    drop(staged);
    assert!(!into.exists(), "the copy is removed");
    assert!(from.join("a.capnp").exists(), "the original stays");
    std::fs::remove_dir_all(&from).unwrap();
}

#[test]
fn a_failing_copy_is_an_error_with_the_path_and_leaves_nothing_behind() {
    let from = scratch("failcopy");
    std::fs::write(from.join("a.capnp"), "@0x1;").unwrap();
    // A folder named like a schema: `fs::copy` cannot copy it.
    std::fs::create_dir(from.join("x.capnp")).unwrap();
    let into = from.join("copy");
    let Err(message) = stage::stage(&from, &into) else {
        panic!("a failing copy is an error")
    };
    assert!(
        message.contains("x.capnp"),
        "the message names the file: {message}"
    );
    assert!(!into.exists(), "the half-filled copy is removed");
    std::fs::remove_dir_all(&from).unwrap();
}

#[test]
fn an_existing_target_is_an_error_and_is_never_removed() {
    let from = scratch("existing");
    std::fs::write(from.join("a.capnp"), "@0x1;").unwrap();
    let into = scratch("existing-into");
    std::fs::write(into.join("keep.txt"), "mine").unwrap();
    let Err(message) = stage::stage(&from, &into) else {
        panic!("an existing target is an error")
    };
    assert!(message.contains("existing-into"), "{message}");
    assert_eq!(
        std::fs::read_to_string(into.join("keep.txt")).unwrap(),
        "mine"
    );
    std::fs::remove_dir_all(&from).unwrap();
    std::fs::remove_dir_all(&into).unwrap();
}

#[test]
fn drives_are_compared_by_letter_only() {
    assert!(stage::same_drive(Some(b'C'), Some(b'C')));
    assert!(!stage::same_drive(Some(b'C'), Some(b'D')));
    assert!(
        stage::same_drive(None, Some(b'D')),
        "no drive = relative to the current one"
    );
    assert!(!stage::same_drive(Some(b'C'), None));
}

#[cfg(not(windows))]
#[test]
fn unix_paths_have_no_drive() {
    assert_eq!(stage::drive_of(Path::new("/home/x")), None);
}

#[cfg(windows)]
#[test]
fn windows_drive_letters_are_read_in_every_form() {
    assert_eq!(stage::drive_of(Path::new(r"c:\x")), Some(b'C'));
    assert_eq!(stage::drive_of(Path::new(r"\\?\D:\x")), Some(b'D'));
    assert_eq!(
        stage::drive_of(Path::new(r"C:/mixed\separators")),
        Some(b'C')
    );
    assert_eq!(stage::drive_of(Path::new(r"relative\x")), None);
}

#[cfg(windows)]
#[test]
fn the_fs_types_schema_is_reachable_from_here_on_windows() {
    // The Cargo checkout may be on another drive than the repository (the runner: C: and D:).
    let r = stage::reachable(Path::new(fs_types::SCHEMA_DIR)).unwrap();
    assert!(
        r.path().join("types.capnp").is_file(),
        "{}",
        r.path().display()
    );
    let cwd = std::env::current_dir().unwrap();
    assert!(stage::same_drive(
        stage::drive_of(r.path()),
        stage::drive_of(&cwd)
    ));
}
