//! This version appends `PackageKind::Module @5` and `PackageKind::Data @6`: the schema before (kept as generated code in
//! `compat/`) reads the new values as "not in my schema" (no crash, the rest of the package is intact), the new schema reads
//! every old value `@0` to `@4` unchanged, and the numbers of all values stay fixed.

#[allow(unused_imports)] // the frozen generated code below refers to `crate::common_capnp`
use fs_interfaces::common_capnp;
use fs_interfaces::store_capnp as new;

#[path = "compat/store_v0_3_0_capnp.rs"]
#[allow(clippy::all, clippy::pedantic, unused_parens, dead_code, missing_docs)]
mod store_capnp;

fn bytes_with_kind(kind: new::PackageKind) -> Vec<u8> {
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut pkg = msg.init_root::<new::package::Builder<'_>>();
        pkg.set_name("org.freesynergy.Tasks");
        pkg.set_version("1.0.0");
        pkg.set_kind(kind);
    }
    capnp::serialize::write_message_to_words(&msg)
}

fn reader_of(bytes: &[u8]) -> capnp::message::Reader<impl capnp::message::ReaderSegments + '_> {
    capnp::serialize::read_message_from_flat_slice(
        &mut &bytes[..],
        capnp::message::ReaderOptions::new(),
    )
    .unwrap()
}

#[test]
fn the_new_schema_roundtrips_module_and_data() {
    for (kind, number) in [(new::PackageKind::Module, 5), (new::PackageKind::Data, 6)] {
        let bytes = bytes_with_kind(kind);
        let reader = reader_of(&bytes);
        let pkg = reader.get_root::<new::package::Reader<'_>>().unwrap();
        assert_eq!(pkg.get_kind().unwrap(), kind);
        assert_eq!(u16::from(kind), number);
    }
}

#[test]
fn the_old_schema_reads_module_and_data_as_unknown_and_the_rest_intact() {
    for (kind, number) in [(new::PackageKind::Module, 5), (new::PackageKind::Data, 6)] {
        let bytes = bytes_with_kind(kind);
        let reader = reader_of(&bytes);
        let old = reader
            .get_root::<store_capnp::package::Reader<'_>>()
            .unwrap();
        assert!(matches!(old.get_kind(), Err(capnp::NotInSchema(n)) if n == number));
        assert_eq!(
            old.get_name().unwrap().to_str().unwrap(),
            "org.freesynergy.Tasks"
        );
        assert_eq!(old.get_version().unwrap().to_str().unwrap(), "1.0.0");
    }
}

#[test]
fn the_old_values_keep_their_numbers_and_read_unchanged() {
    for (old_kind, expected, number) in [
        (
            store_capnp::PackageKind::Program,
            new::PackageKind::Program,
            0,
        ),
        (
            store_capnp::PackageKind::Frontend,
            new::PackageKind::Frontend,
            1,
        ),
        (
            store_capnp::PackageKind::Component,
            new::PackageKind::Component,
            2,
        ),
        (store_capnp::PackageKind::Theme, new::PackageKind::Theme, 3),
        (
            store_capnp::PackageKind::Server,
            new::PackageKind::Server,
            4,
        ),
    ] {
        assert_eq!(u16::from(old_kind), number);
        assert_eq!(u16::from(expected), number);
        let mut msg = capnp::message::Builder::new_default();
        msg.init_root::<store_capnp::package::Builder<'_>>()
            .set_kind(old_kind);
        let bytes = capnp::serialize::write_message_to_words(&msg);
        let reader = reader_of(&bytes);
        let pkg = reader.get_root::<new::package::Reader<'_>>().unwrap();
        assert_eq!(pkg.get_kind().unwrap(), expected);
    }
}
