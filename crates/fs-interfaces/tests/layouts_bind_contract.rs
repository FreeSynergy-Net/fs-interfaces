//! The contracts `layouts` and `bind` (draft, owner fs-forms, since 0.0.28): final file ids, the modules exist, the capability is
//! on the list, and every schema file is covered by `generated_matches_official` and listed in `regenerate`.

use std::path::{Path, PathBuf};

use fs_interfaces::bind_capnp::bind;
use fs_interfaces::capabilities;
use fs_interfaces::layouts_capnp::{layout_admin, layouts};

/// The final ids; the copies in `fs-rpc` carried provisional ones (`0xbf2feddf75510ad3`, `0xd1a5c3e7b9f20468`).
const LAYOUTS_ID: &str = "@0x92216eab3844e15f;";
const BIND_ID: &str = "@0xf8c4a79309b02c91;";

fn schema_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schema")
}

/// The file id of a schema: the first line that is not a comment.
fn file_id(name: &str) -> String {
    let text = std::fs::read_to_string(schema_dir().join(name)).unwrap();
    let line = text
        .lines()
        .find(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .unwrap();
    line.split_whitespace().next().unwrap().to_owned()
}

#[test]
fn the_file_ids_are_final_and_unique() {
    assert_eq!(file_id("layouts.capnp"), LAYOUTS_ID);
    assert_eq!(file_id("bind.capnp"), BIND_ID);
    let mut ids = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(schema_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "capnp") {
            let name = path.file_name().unwrap().to_str().unwrap().to_owned();
            let id = file_id(&name);
            assert!(id.starts_with("@0x"), "{name}: {id}");
            if let Some(other) = ids.insert(id.clone(), name.clone()) {
                panic!("{name} and {other} share the file id {id}");
            }
        }
    }
}

#[test]
fn the_modules_exist() {
    assert!(std::any::type_name::<layouts::Client>().contains("layouts"));
    assert!(std::any::type_name::<layout_admin::Client>().contains("layout_admin"));
    assert!(std::any::type_name::<bind::Client>().contains("bind"));
}

#[test]
fn the_capability_layouts_is_planned() {
    assert!(capabilities::is_known("layouts"));
    assert!(capabilities::is_planned("layouts"));
    assert_eq!(capabilities::display_name("layouts"), Some("Layouts"));
}

#[test]
fn every_schema_file_is_checked_and_regenerated() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let checked =
        std::fs::read_to_string(manifest.join("tests/generated_matches_official.rs")).unwrap();
    let regenerate = std::fs::read_to_string(manifest.join("../regenerate/src/main.rs")).unwrap();
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(schema_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "capnp") {
            let stem = path.file_stem().unwrap().to_str().unwrap().to_owned();
            let line = format!("    \"{stem}\",");
            if !checked.contains(&line) {
                missing.push(format!("{stem} (generated_matches_official)"));
            }
            if !regenerate.contains(&line) {
                missing.push(format!("{stem} (regenerate)"));
            }
        }
    }
    assert!(missing.is_empty(), "schema files not covered: {missing:?}");
}

#[test]
fn the_instance_contract_is_there() {
    // draft, owner fs-starter (ADR 0097): the file id is final, the module exists
    assert_eq!(file_id("instance.capnp"), "@0xa36ccf61f34f0d59;");
    assert!(
        std::any::type_name::<fs_interfaces::instance_capnp::instance::Client>()
            .contains("instance")
    );
}
