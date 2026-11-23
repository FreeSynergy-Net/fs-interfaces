//! The count of a channel (draft form catalog): no `?` anywhere, the reader refuses it with the line, and `Count::allows` is the check
//! the matcher uses for how many fields a module maps onto a channel.

use fs_interfaces::forms::{self, Count, ErrorKind, parse};

const GOOD: &str = "a@1.0.0 | A | planned | text\n> items | records | ! | 1\n";

fn kind(text: &str) -> ErrorKind {
    parse(text).unwrap_err().kind
}

/// No channel of the catalog has an open count (`?`); the reader refuses it with the line.
#[test]
fn no_count_is_open_and_the_reader_refuses_a_question_mark() {
    assert!(
        !forms::SOURCE
            .lines()
            .any(|l| l.starts_with('>') && l.trim_end().ends_with("| ?"))
    );
    let e = parse(&format!("{GOOD}> x | text | - | ?\n")).unwrap_err();
    assert_eq!((e.line, e.kind), (3, ErrorKind::Count));
    assert_eq!(
        kind(&format!("{GOOD}> x | text | - | 0..\n")),
        ErrorKind::Count
    );
    let cic = forms::find_key("cards-in-columns@1.0.0").unwrap();
    assert_eq!(
        cic.channel("cards").unwrap().count,
        Count {
            min: 1,
            max: Some(1)
        }
    );
    assert_eq!(
        cic.channel("lane").unwrap().count,
        Count {
            min: 0,
            max: Some(1)
        }
    );
}

#[test]
fn a_count_allows_the_numbers_inside_it() {
    let facts = forms::find_key("tiles@1.0.0")
        .unwrap()
        .channel("facts")
        .unwrap()
        .count;
    assert!(facts.allows(0) && facts.allows(4) && !facts.allows(5));
    let one = Count {
        min: 1,
        max: Some(1),
    };
    assert!(!one.allows(0) && one.allows(1) && !one.allows(2));
    let many = Count { min: 0, max: None };
    assert!(many.allows(0) && many.allows(1_000_000));
    let from_one = parse("a@1.0.0 | A | planned | t\n> items | records | ! | 1..n\n").unwrap();
    assert!(!from_one[0].channels[0].count.allows(0) && from_one[0].channels[0].count.allows(7));
}
