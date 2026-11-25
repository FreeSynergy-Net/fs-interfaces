//! L2a (ENTWURF lanes source, format 4): `Binding` of `layouts.capnp` carries `source @9` and `hasSource @10` (appended only), and
//! `Source.scope` is a free text token. Old writer, new reader and new writer, old reader must both read.

use capnp::traits::HasStructSize;
use fs_interfaces::layouts_capnp::{binding, source};

fn words(msg: &capnp::message::Builder<capnp::message::HeapAllocator>) -> Vec<u8> {
    capnp::serialize::write_message_to_words(msg)
}

fn read(bytes: &[u8]) -> capnp::message::Reader<capnp::serialize::BufferSegments<&[u8]>> {
    capnp::serialize::read_message_from_flat_slice(
        &mut &bytes[..],
        capnp::message::ReaderOptions::new(),
    )
    .unwrap()
}

fn write(with_source: Option<u32>) -> Vec<u8> {
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut b = msg.init_root::<binding::Builder<'_>>();
        b.set_elem("binding:board-swimlanes/main/lanes/0");
        b.set_slot("lanes");
        b.set_from("grouping");
        b.set_name("assignee");
        if let Some(n) = with_source {
            b.set_source(n);
            b.set_has_source(true);
        }
    }
    words(&msg)
}

#[test]
fn a_message_of_an_old_writer_reads_no_source() {
    let bytes = write(None);
    let msg = read(&bytes);
    let b = msg.get_root::<binding::Reader<'_>>().unwrap();
    assert!(!b.get_has_source());
    assert_eq!(b.get_source(), 0);
    assert_eq!(b.get_name().unwrap().to_str().unwrap(), "assignee");
}

#[test]
fn source_zero_is_told_apart_from_no_source() {
    for n in [0u32, 2] {
        let bytes = write(Some(n));
        let msg = read(&bytes);
        let b = msg.get_root::<binding::Reader<'_>>().unwrap();
        assert!(b.get_has_source());
        assert_eq!(b.get_source(), n);
    }
}

/// "Old reader reads new message": the pointer section (7) and the old data word stay; the struct only GROWS at the end (data 1 -> 2 words,
/// `source`/`hasSource` sit behind `nth`/`hasSchemes`). Old accessors answer the same on a new message.
#[test]
fn the_old_fields_keep_their_place_and_only_the_end_grows() {
    let size = <binding::Owned as capnp::traits::OwnedStruct>::Builder::STRUCT_SIZE;
    assert_eq!((size.data, size.pointers), (2, 7));
    let bytes = write(Some(2));
    let msg = read(&bytes);
    let b = msg.get_root::<binding::Reader<'_>>().unwrap();
    assert_eq!(b.get_slot().unwrap().to_str().unwrap(), "lanes");
    assert_eq!(b.get_from().unwrap().to_str().unwrap(), "grouping");
    assert_eq!(b.get_name().unwrap().to_str().unwrap(), "assignee");
    assert!(!b.get_has_schemes());
    assert_eq!(b.get_schemes().unwrap().len(), 0);
    assert_eq!(b.get_nth(), 0);
    let old = write(None);
    assert_eq!(
        bytes.len(),
        old.len(),
        "same message size: the new numbers live in the added data word"
    );
}

#[test]
fn a_source_scope_is_a_free_token_on_the_wire() {
    let mut msg = capnp::message::Builder::new_default();
    msg.init_root::<source::Builder<'_>>()
        .set_scope("lanes:assignee:board-1");
    let bytes = words(&msg);
    let m = read(&bytes);
    let s = m.get_root::<source::Reader<'_>>().unwrap();
    assert_eq!(
        s.get_scope().unwrap().to_str().unwrap(),
        "lanes:assignee:board-1"
    );
}
