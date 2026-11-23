//! The name guard on its own: traversal and control characters are refused before any provider sees them.

use fs_storage::{Error, NameProblem, split_path, validate_name};

fn why(name: &str) -> NameProblem {
    match validate_name(name) {
        Err(Error::InvalidName { reason, .. }) => reason,
        other => panic!("{name:?} was accepted or wrongly refused: {other:?}"),
    }
}

#[test]
fn refuses_what_could_escape_or_hide() {
    assert_eq!(why(""), NameProblem::Empty);
    assert_eq!(why(".."), NameProblem::Dot);
    assert_eq!(why("a/b"), NameProblem::Separator);
    assert_eq!(why("a\\b"), NameProblem::Separator);
    assert_eq!(why("a\0b"), NameProblem::Control);
    assert_eq!(why("a\u{9b}b"), NameProblem::Control);
    assert_eq!(why("gpj.\u{202E}exe"), NameProblem::Bidi);
    assert_eq!(why(&"x".repeat(256)), NameProblem::TooLong);
    assert_eq!(why(".fs-files"), NameProblem::Reserved);
}

#[test]
fn accepts_normal_names_and_paths() {
    assert!(validate_name("Bericht (2).pdf").is_ok());
    assert!(validate_name(&"x".repeat(255)).is_ok());
    assert!(validate_name("..hidden").is_ok());
    assert_eq!(split_path("a/b/c").unwrap(), ["a", "b", "c"]);
    assert_eq!(split_path("").unwrap(), Vec::<&str>::new());
    for bad in ["/a", "a//b", "a/../b", "a/."] {
        assert!(split_path(bad).is_err(), "{bad}");
    }
}

#[test]
fn problems_carry_type_and_status() {
    let e = Error::Forbidden {
        capability: "storage.read".into(),
    };
    assert_eq!(e.status(), 403);
    assert!(e.problem_type().ends_with("/storage/forbidden"));
    assert_eq!(Error::NameConflict { name: "x".into() }.status(), 409);
    assert_eq!(Error::Cycle(fs_storage::EntryId("x".into())).status(), 508);
}
