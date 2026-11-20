//! v0.0.29 appends the methods of `Boards` to `tasks.capnp` (ADR 0104 E3): the file id and the type ids of everything that
//! was there stay, no existing method moves, and the new calls and structs go through the wire and come back unchanged.
//! Pattern: none (tests).

use capnp::message::{Builder, ReaderOptions};
use capnp::traits::HasTypeId;
use fs_interfaces::tasks_capnp::{
    Readiness, Relation, board, board_column, board_layout, boards, column_layout, items,
    placement, task, task_page, tasks,
};

const SCHEMA: &str = include_str!("../../../schema/tasks.capnp");

/// Methods of an interface as written in the schema: `name @N (`, from `interface <name> {` to its closing line.
fn methods(interface: &str) -> Vec<(String, u32)> {
    let head = format!("interface {interface} {{");
    let body = SCHEMA.split(&head).nth(1).expect("interface exists");
    let body = body.split("\n}").next().expect("interface ends");
    body.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .filter_map(|l| {
            let (name, rest) = l.trim().split_once(" @")?;
            let (n, after) = rest.split_once(' ')?;
            if !after.trim_start().starts_with('(') {
                return None;
            }
            Some((name.trim().to_owned(), n.parse().ok()?))
        })
        .collect()
}

fn named(list: &[(&str, u32)]) -> Vec<(String, u32)> {
    list.iter().map(|(n, o)| ((*n).to_owned(), *o)).collect()
}

#[test]
fn the_file_id_and_the_type_ids_of_what_was_there_are_unchanged() {
    assert!(SCHEMA.contains("@0xc04d258971543476;"), "file id");
    assert_eq!(tasks::Client::TYPE_ID, 0xb8a9_8bc6_d030_a118);
    assert_eq!(items::Client::TYPE_ID, 0xf3af_bd16_50ce_9661);
    assert_eq!(
        boards::Client::TYPE_ID,
        0xdae6_a323_bf43_c7ca,
        "Boards is the interface of the placeholder"
    );
    assert_eq!(task::Reader::TYPE_ID, 0xd78b_38b6_fba7_b0ca);
    assert_eq!(task_page::Reader::TYPE_ID, 0xdc99_9b89_e379_6107);
    assert_eq!(Readiness::TYPE_ID, 0xbb9a_4cf5_2441_5c3e);
    assert_eq!(Relation::TYPE_ID, 0xda6e_5db7_6351_d75b);
}

#[test]
fn existing_methods_keep_their_ordinals_and_the_new_ones_are_appended() {
    assert_eq!(
        methods("Tasks"),
        named(&[
            ("capabilities", 0),
            ("items", 1),
            ("subscribe", 2),
            ("boards", 3),
            ("supportReport", 4),
            ("types", 5),
        ])
    );
    let items_before = [
        ("create", 0),
        ("edit", 1),
        ("assign", 2),
        ("createRelation", 3),
        ("assignLabel", 4),
        ("get", 5),
        ("getByUid", 6),
        ("delete", 7),
        ("list", 8),
        ("query", 9),
        ("importIcs", 10),
        ("exportIcs", 11),
        ("editIf", 12),
        ("readiness", 13),
        ("blockedBy", 14),
        ("dependents", 15),
        ("createRelationWithGap", 16),
    ];
    assert_eq!(methods("Items"), named(&items_before));
    // `Boards` had no method: all five are new, from 0 on, in this order
    assert_eq!(
        methods("Boards"),
        named(&[
            ("create", 0),
            ("list", 1),
            ("layout", 2),
            ("editColumns", 3),
            ("moveCard", 4),
        ])
    );
}

fn read<O: capnp::traits::Owned>(
    msg: &Builder<capnp::message::HeapAllocator>,
    f: impl for<'a> FnOnce(O::Reader<'a>),
) {
    let bytes = capnp::serialize::write_message_to_words(msg);
    let reader =
        capnp::serialize::read_message_from_flat_slice(&mut &bytes[..], ReaderOptions::new())
            .unwrap();
    f(reader.get_root::<O::Reader<'_>>().unwrap());
}

fn fill_board(mut b: board::Builder<'_>) {
    b.set_name("Sprint");
    b.set_label("sprint-7");
    b.set_revision(u64::MAX);
    let mut cols = b.reborrow().init_columns(2);
    {
        let mut c = cols.reborrow().get(0);
        c.set_id("todo");
        c.set_name("To do");
        c.set_wip_limit(3);
        let mut p = c.init_progress(1);
        p.set(0, "needs-action");
    }
    {
        let mut c = cols.reborrow().get(1);
        c.set_id("done");
        c.set_name("Done");
    }
    let mut order = b.init_order(1);
    let mut one = order.reborrow().get(0);
    one.set_column("todo");
    let mut cards = one.init_cards(2);
    cards.set(0, "uid-a");
    cards.set(1, "uid-b");
}

fn check_board(b: board::Reader<'_>) {
    assert_eq!(b.get_name().unwrap().to_str().unwrap(), "Sprint");
    assert_eq!(b.get_label().unwrap().to_str().unwrap(), "sprint-7");
    assert_eq!(b.get_revision(), u64::MAX);
    let cols = b.get_columns().unwrap();
    assert_eq!(cols.len(), 2);
    assert_eq!(cols.get(0).get_wip_limit(), 3);
    assert_eq!(cols.get(0).get_id().unwrap().to_str().unwrap(), "todo");
    assert_eq!(
        cols.get(0)
            .get_progress()
            .unwrap()
            .get(0)
            .unwrap()
            .to_str()
            .unwrap(),
        "needs-action"
    );
    assert_eq!(cols.get(1).get_wip_limit(), 0, "0 = no limit, also unset");
    assert!(cols.get(1).get_progress().unwrap().is_empty());
    let order = b.get_order().unwrap();
    assert_eq!(order.get(0).get_column().unwrap().to_str().unwrap(), "todo");
    let cards = order.get(0).get_cards().unwrap();
    assert_eq!(cards.len(), 2);
    assert_eq!(cards.get(1).unwrap().to_str().unwrap(), "uid-b");
}

#[test]
fn a_board_goes_through_the_wire_with_every_field() {
    let mut msg = Builder::new_default();
    fill_board(msg.init_root::<board::Builder<'_>>());
    read::<board::Owned>(&msg, check_board);
}

#[test]
fn the_params_and_results_of_every_method_go_through_the_wire() {
    let mut msg = Builder::new_default();
    {
        let mut p = msg.init_root::<boards::create_params::Builder<'_>>();
        p.set_name("Sprint");
        p.set_label("");
    }
    read::<boards::create_params::Owned>(&msg, |p| {
        assert_eq!(p.get_name().unwrap().to_str().unwrap(), "Sprint");
        assert!(
            p.get_label().unwrap().is_empty(),
            "empty label = every task"
        );
    });

    let mut msg = Builder::new_default();
    fill_board(
        msg.init_root::<boards::list_results::Builder<'_>>()
            .init_boards(1)
            .get(0),
    );
    read::<boards::list_results::Owned>(&msg, |r| check_board(r.get_boards().unwrap().get(0)));

    let mut msg = Builder::new_default();
    {
        let mut p = msg.init_root::<boards::edit_columns_params::Builder<'_>>();
        p.set_revision(7);
        let mut cols = p.reborrow().init_columns(1);
        cols.reborrow().get(0).set_id("c1");
        cols.reborrow().get(0).set_wip_limit(2);
        let mut order = p.init_order(1);
        order.reborrow().get(0).set_column("c1");
        order.get(0).init_cards(1).set(0, "u1");
    }
    read::<boards::edit_columns_params::Owned>(&msg, |p| {
        assert_eq!(p.get_revision(), 7);
        assert_eq!(p.get_columns().unwrap().get(0).get_wip_limit(), 2);
        let cards = p.get_order().unwrap().get(0).get_cards().unwrap();
        assert_eq!(cards.get(0).unwrap().to_str().unwrap(), "u1");
    });

    let mut msg = Builder::new_default();
    {
        let mut p = msg.init_root::<boards::move_card_params::Builder<'_>>();
        p.set_uid("u1");
        p.set_column("done");
        p.set_index(-1);
    }
    read::<boards::move_card_params::Owned>(&msg, |p| {
        assert_eq!(p.get_uid().unwrap().to_str().unwrap(), "u1");
        assert_eq!(p.get_column().unwrap().to_str().unwrap(), "done");
        assert_eq!(p.get_index(), -1, "negative = at the end");
    });
}

#[test]
fn a_layout_goes_through_the_wire_with_the_excess_report() {
    let mut msg = Builder::new_default();
    {
        let layout = msg
            .init_root::<boards::layout_results::Builder<'_>>()
            .init_layout();
        let mut cols = layout.init_columns(1);
        let mut c = cols.reborrow().get(0);
        c.reborrow().init_column().set_id("todo");
        c.reborrow().init_cards(1).set(0, "u1");
        c.set_over_limit(true);
    }
    read::<boards::layout_results::Owned>(&msg, |r| {
        let cols = r.get_layout().unwrap().get_columns().unwrap();
        let c = cols.get(0);
        assert!(c.get_over_limit());
        assert_eq!(
            c.get_column().unwrap().get_id().unwrap().to_str().unwrap(),
            "todo"
        );
        assert_eq!(
            c.get_cards().unwrap().get(0).unwrap().to_str().unwrap(),
            "u1"
        );
    });
    // the unset report is `false`: nothing is over a limit that is not there
    let mut msg = Builder::new_default();
    msg.init_root::<column_layout::Builder<'_>>();
    read::<column_layout::Owned>(&msg, |c| assert!(!c.get_over_limit()));
    let _ = (
        board_column::Reader::TYPE_ID,
        placement::Reader::TYPE_ID,
        board_layout::Reader::TYPE_ID,
    );
}
