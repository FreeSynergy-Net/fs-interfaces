//! The capability list (ADR 0068): one file, read here; the names are well formed, unique and the constants are on it.
use fs_interfaces::capabilities::{self, name};

#[test]
fn the_list_is_well_formed_and_unique() {
    let all: Vec<_> = capabilities::all().collect();
    assert!(all.len() >= 20);
    let mut seen = std::collections::BTreeSet::new();
    for c in &all {
        assert!(capabilities::is_well_formed(c.name), "{}", c.name);
        assert!(!c.description.is_empty(), "{} has no description", c.name);
        assert!(!c.display_name.is_empty(), "{} has no display name", c.name);
        assert!(seen.insert(c.name), "{} twice", c.name);
    }
}

#[test]
fn constants_are_on_the_list() {
    for n in [
        name::PEOPLE_PERSONS,
        name::PEOPLE_USERS,
        name::GROUPS,
        name::COMMENTS,
        name::DEPENDENCY_EVALUATE,
        name::BOARD_OPEN,
        name::BOARD_CREATE,
        name::COLUMN_CREATE,
        name::COLUMN_LIMIT,
        name::CARD_MOVE,
        name::STORAGE_PROVIDER,
        name::IDENTITY_OIDC,
        name::IDENTITY_ACCOUNTS,
        name::IDENTITY_LDAP,
        name::IDENTITY_SCIM,
        name::IDENTITY_PAM,
        name::IDENTITY_RADIUS,
        name::IDENTITY_OAUTH2,
        name::IDENTITY_SSH_KEYS,
        name::IDENTITY_PASSKEYS,
        name::GATEWAY_REVERSE_PROXY,
        name::MESSAGES_MATRIX,
        name::MESSAGES_MATRIX_OIDC_SERVER,
    ] {
        assert!(capabilities::is_known(n), "{n}");
    }
    assert!(!capabilities::is_known("people.persons.x"));
    assert!(!capabilities::is_well_formed("People"));
    assert!(!capabilities::is_well_formed("a..b"));
    assert!(!capabilities::is_well_formed(""));
}

#[test]
fn every_line_of_the_file_is_read() {
    let lines = capabilities::SOURCE
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .count();
    assert_eq!(
        lines,
        capabilities::all().count(),
        "a line has the wrong form"
    );
}

#[test]
fn display_names_pair_with_the_schema_keys_and_planned_is_marked() {
    assert_eq!(capabilities::display_name("whiteboard"), Some("Boards"));
    assert_eq!(capabilities::display_name("surveys"), Some("Forms"));
    assert_eq!(capabilities::display_name("messenger"), Some("Messages"));
    assert_eq!(capabilities::display_name("boards"), None);
    assert!(capabilities::is_planned(name::STORAGE_PROVIDER));
    assert!(capabilities::is_known(name::STORAGE_PROVIDER));
    for n in [
        name::IDENTITY_OAUTH2,
        name::IDENTITY_SSH_KEYS,
        name::IDENTITY_PASSKEYS,
        name::GATEWAY_REVERSE_PROXY,
        name::MESSAGES_MATRIX,
        name::MESSAGES_MATRIX_OIDC_SERVER,
    ] {
        assert!(capabilities::is_planned(n), "{n}");
    }
    assert_eq!(
        capabilities::display_name(name::IDENTITY_OAUTH2),
        Some("OAuth 2.0")
    );
    assert!(!capabilities::is_planned(name::COMMENTS));
    assert!(!capabilities::is_planned("nonsense"));
}

#[test]
fn me_create_is_on_the_list_as_stable() {
    assert_eq!(name::ME_CREATE, "me.create");
    assert!(capabilities::is_known(name::ME_CREATE));
    assert!(!capabilities::is_planned(name::ME_CREATE));
    assert!(capabilities::is_well_formed(name::ME_CREATE));
    // `me` is no topic key of its own: only the one command key is on the list.
    assert!(!capabilities::is_known("me"));
}

#[test]
fn the_person_crud_commands_are_on_the_list_as_stable() {
    for (constant, key) in [
        (name::PERSON_PROFILE_CREATE, "person.profile.create"),
        (name::PERSON_UPDATE, "person.update"),
        (name::PERSON_DELETE, "person.delete"),
    ] {
        assert_eq!(constant, key);
        assert!(capabilities::is_known(key), "{key}");
        assert!(!capabilities::is_planned(key), "{key}");
        assert!(capabilities::is_well_formed(key), "{key}");
    }
    // only the three keys are on the list, not a made-up fourth
    assert!(!capabilities::is_known("person.profile.delete"));
    // `task.edit` was on the list already (the planning module names it, nothing to append)
    assert!(capabilities::is_known("task.edit"));
}
