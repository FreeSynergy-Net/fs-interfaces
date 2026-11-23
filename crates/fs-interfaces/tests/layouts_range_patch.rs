//! V-2 (ENTSCHEID-schnitt): `Range` of `layouts.capnp` carries the third number of both bounds (`minPatch @6`, `belowPatch @7`), appended only.
//! A range like `>=0.0.1, <0.1.0` must pass the wire without a silent cut, and the old fields keep their place in the message.

use fs_interfaces::layouts_capnp::{requires, source};

const TEXT: &str = "tasks@>=0.0.1, <0.1.0";

fn write(patch: bool) -> Vec<u8> {
    let mut msg = capnp::message::Builder::new_default();
    {
        let req = msg.init_root::<requires::Builder<'_>>();
        let mut range = req.init_range();
        range.set_capability("tasks");
        range.set_min_major(0);
        range.set_min_minor(0);
        range.set_below_major(0);
        range.set_below_minor(1);
        if patch {
            range.set_min_patch(1);
            range.set_below_patch(0);
        }
        range.set_text(TEXT);
    }
    capnp::serialize::write_message_to_words(&msg)
}

#[test]
fn a_range_with_a_patch_goes_through_the_wire_without_loss() {
    let bytes = write(true);
    let msg = capnp::serialize::read_message_from_flat_slice(
        &mut bytes.as_slice(),
        capnp::message::ReaderOptions::new(),
    )
    .unwrap();
    let range = msg
        .get_root::<requires::Reader<'_>>()
        .unwrap()
        .get_range()
        .unwrap();
    assert_eq!(
        (
            range.get_min_major(),
            range.get_min_minor(),
            range.get_min_patch()
        ),
        (0, 0, 1)
    );
    assert_eq!(
        (
            range.get_below_major(),
            range.get_below_minor(),
            range.get_below_patch()
        ),
        (0, 1, 0)
    );
    assert_eq!(range.get_text().unwrap().to_str().unwrap(), TEXT);
}

#[test]
fn the_patch_of_the_bounds_is_kept_apart_from_the_minor() {
    let mut msg = capnp::message::Builder::new_default();
    {
        let mut range = msg.init_root::<source::Builder<'_>>().init_range();
        range.set_min_minor(2);
        range.set_min_patch(7);
        range.set_below_minor(3);
        range.set_below_patch(9);
    }
    let range = msg
        .get_root_as_reader::<source::Reader<'_>>()
        .unwrap()
        .get_range()
        .unwrap();
    assert_eq!((range.get_min_minor(), range.get_min_patch()), (2, 7));
    assert_eq!((range.get_below_minor(), range.get_below_patch()), (3, 9));
}

#[test]
fn a_message_of_an_old_writer_reads_patch_zero() {
    let bytes = write(false);
    let msg = capnp::serialize::read_message_from_flat_slice(
        &mut bytes.as_slice(),
        capnp::message::ReaderOptions::new(),
    )
    .unwrap();
    let range = msg
        .get_root::<requires::Reader<'_>>()
        .unwrap()
        .get_range()
        .unwrap();
    assert_eq!((range.get_min_patch(), range.get_below_patch()), (0, 0));
    assert_eq!(range.get_min_minor(), 0);
    assert_eq!(range.get_below_minor(), 1);
}

/// "Old reader reads new message": the four old numbers keep their slots (u32 index 0..3 of the data section), the text its pointer
/// slot; the patch numbers sit behind them (appended). The sizes of the struct are only allowed to GROW (data 2 -> 3 words).
#[test]
fn the_old_fields_keep_their_place_and_only_the_end_grows() {
    use capnp::traits::HasStructSize;
    let size = <fs_interfaces::layouts_capnp::range::Owned as capnp::traits::OwnedStruct>::Builder::STRUCT_SIZE;
    assert_eq!((size.data, size.pointers), (3, 2));

    let bytes = write(true);
    // Segment table (8 bytes), root pointer (8 bytes), `Requires` data (0 words) + 2 pointers: `elem`, `range`; the range struct
    // follows its pointer, so find it by the numbers: min 0.0 / below 0.1 sit in the first 16 data bytes of the range struct.
    let words: Vec<u64> = bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|c| u64::from_le_bytes(*c))
        .collect();
    // old layout: word0 = minMajor | minMinor << 32, word1 = belowMajor | belowMinor << 32, word2 (new) = minPatch | belowPatch << 32
    words
        .windows(3)
        .position(|w| w[0] == 0 && w[1] == (1u64 << 32) && w[2] == 1)
        .expect("old numbers in the old slots, patch behind them");
}
