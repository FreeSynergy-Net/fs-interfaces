//! `Action` of `layouts.capnp` carries `at @8` (appended only): the element id of the place where the action applies.
//! Pattern: Specification of the wire rule "append only, read tolerantly". Round trip, old bytes, and the guard that fails when
//! a writer loses `at`.

use capnp::traits::HasStructSize;
use fs_interfaces::layouts_capnp::action;

/// Bytes written by the writer BEFORE `at` existed (pointers 6, data 1 word): elem, n 1, trigger drop, command card.move, confirm.
const OLD_BYTES: &[u8] = &[
    0x00, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x06, 0x00,
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x15, 0x00, 0x00, 0x00, 0x7a, 0x00, 0x00, 0x00,
    0x19, 0x00, 0x00, 0x00, 0x2a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x15, 0x00, 0x00, 0x00, 0x52, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x15, 0x00, 0x00, 0x00, 0x42, 0x00, 0x00, 0x00, 0x61, 0x63, 0x74, 0x69, 0x6f, 0x6e, 0x3a, 0x62,
    0x6f, 0x61, 0x72, 0x64, 0x2f, 0x30, 0x00, 0x00, 0x64, 0x72, 0x6f, 0x70, 0x00, 0x00, 0x00, 0x00,
    0x63, 0x61, 0x72, 0x64, 0x2e, 0x6d, 0x6f, 0x76, 0x65, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x63, 0x6f, 0x6e, 0x66, 0x69, 0x72, 0x6d, 0x00,
];

fn read(bytes: &[u8]) -> capnp::message::Reader<capnp::serialize::OwnedSegments> {
    capnp::serialize::read_message(&mut &bytes[..], capnp::message::ReaderOptions::new()).unwrap()
}

/// The writer under test: the same shape as the old bytes, plus `at` when given.
fn write(at: Option<&str>) -> Vec<u8> {
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut a = msg.init_root::<action::Builder<'_>>();
        a.set_elem("action:board/0");
        a.set_n(1);
        a.set_trigger("drop");
        a.set_command("card.move");
        a.set_confirm("confirm");
        if let Some(at) = at {
            a.set_at(at);
        }
    }
    capnp::serialize::write_message_to_words(&msg)
}

#[test]
fn an_action_with_at_makes_the_round_trip() {
    let bytes = write(Some("block:board/main"));
    let msg = read(&bytes);
    let a = msg.get_root::<action::Reader<'_>>().unwrap();
    assert_eq!(a.get_at().unwrap().to_str().unwrap(), "block:board/main");
    assert_eq!(a.get_trigger().unwrap().to_str().unwrap(), "drop");
    assert_eq!(a.get_command().unwrap().to_str().unwrap(), "card.move");
    assert_eq!(a.get_confirm().unwrap().to_str().unwrap(), "confirm");
    assert_eq!(a.get_n(), 1);
}

#[test]
fn old_bytes_read_with_an_empty_at_and_unchanged_fields() {
    let msg = read(OLD_BYTES);
    let a = msg.get_root::<action::Reader<'_>>().unwrap();
    assert_eq!(a.get_at().unwrap().to_str().unwrap(), "");
    assert_eq!(a.get_elem().unwrap().to_str().unwrap(), "action:board/0");
    assert_eq!(a.get_trigger().unwrap().to_str().unwrap(), "drop");
    assert_eq!(a.get_command().unwrap().to_str().unwrap(), "card.move");
    assert_eq!(a.get_confirm().unwrap().to_str().unwrap(), "confirm");
    assert_eq!(a.get_n(), 1);
}

#[test]
fn a_writer_without_at_reads_back_empty() {
    let bytes = write(None);
    let msg = read(&bytes);
    let a = msg.get_root::<action::Reader<'_>>().unwrap();
    assert_eq!(a.get_at().unwrap().to_str().unwrap(), "");
    assert_eq!(a.get_command().unwrap().to_str().unwrap(), "card.move");
}

/// The old pointers keep their place, only the end grows (pointer section 6 -> 7, `at` is the last pointer).
#[test]
fn the_old_fields_keep_their_place_and_only_the_end_grows() {
    let size = <action::Owned as capnp::traits::OwnedStruct>::Builder::STRUCT_SIZE;
    assert_eq!((size.data, size.pointers), (1, 7));
}

/// Guard: a writer that drops `at` must turn this red (the bytes with `at` must differ from the bytes without it and carry the text).
#[test]
fn losing_at_in_the_writer_is_caught() {
    let with = write(Some("block:board/main"));
    let without = write(None);
    assert_ne!(with, without);
    let text = b"block:board/main";
    assert!(with.windows(text.len()).any(|w| w == text));
    assert!(!without.windows(text.len()).any(|w| w == text));
}
