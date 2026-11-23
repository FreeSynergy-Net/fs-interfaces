//! The form catalog (draft): the built-in file parses, is consistent, the reader refuses bad files, and the contract `FormSource`
//! exists with its own file id while the ids of the older contracts stay as they were.

use std::path::{Path, PathBuf};

use fs_interfaces::forms::{self, Art, Count, ErrorKind, Status, Version, parse};

const GOOD: &str = "a@1.0 | A | planned | text\n> items | records | ! | 1\n";

fn kind(text: &str) -> ErrorKind {
    parse(text).unwrap_err().kind
}

#[test]
fn the_builtin_file_parses_with_the_named_forms() {
    let all = parse(forms::SOURCE).unwrap();
    assert_eq!(all.len(), forms::catalog().len());
    for key in [
        "bars-in-time@1.0",
        "tiles@1.0",
        "tree@1.0",
        "members@1.0",
        "checklist@1.0",
        "cards-in-columns@1.0",
    ] {
        assert!(forms::find_key(key).is_some(), "{key}");
    }
    let tiles = forms::find("tiles", Version { major: 1, minor: 0 }).unwrap();
    assert_eq!(tiles.status, Status::Planned);
    let required: Vec<_> = tiles.required().map(|c| c.name.as_str()).collect();
    assert_eq!(required, ["items", "label"]);
    assert_eq!(
        tiles.channel("facts").unwrap().count,
        Count {
            min: 0,
            max: Some(4)
        }
    );
    assert_eq!(tiles.key(), "tiles@1.0");
    assert!(forms::find_key("tiles@2.0").is_none());
    assert!(forms::find_key("tiles").is_none());
}

/// Only the seven basic kinds are in the file: older words (grouping, order, any) were folded into text and number.
#[test]
fn only_the_seven_arts_are_used() {
    let seven = [
        "records",
        "text",
        "number",
        "flag",
        "time",
        "reference",
        "command",
    ];
    for f in forms::catalog() {
        for c in &f.channels {
            for a in &c.arts {
                assert!(seven.contains(&a.name()), "{}.{}", f.key(), c.name);
            }
        }
    }
    let tree = forms::find_key("cards-in-columns@1.0").unwrap();
    assert_eq!(tree.channel("column").unwrap().arts, [Art::Text]);
    let facts = tree.channel("facts").unwrap();
    assert_eq!(facts.arts, [Art::Text, Art::Number, Art::Time, Art::Flag]);
    assert!(facts.accepts(Art::Time) && !facts.accepts(Art::Records));
    assert!(
        facts.accepts_basic("DateTime")
            && facts.accepts_basic("Bool")
            && !facts.accepts_basic("Image")
    );
    assert!(!tree.channel("title").unwrap().accepts_basic("Number"));
    assert_eq!(
        forms::find_key("checklist@1.0")
            .unwrap()
            .channel("order")
            .unwrap()
            .arts,
        [Art::Number]
    );
}

#[test]
fn every_form_is_consistent() {
    let mut keys = std::collections::BTreeSet::new();
    for f in forms::catalog() {
        assert!(keys.insert(f.key()), "{} twice", f.key());
        assert!(f.required().count() >= 1, "{}", f.key());
        for c in &f.channels {
            if c.required {
                assert_ne!(
                    c.count,
                    Count { min: 0, max: None },
                    "{}.{}",
                    f.key(),
                    c.name
                );
            }
        }
        let items = f.channels.first().unwrap();
        assert!(
            items.arts == [Art::Records] && items.required,
            "{}: first channel is the record set",
            f.key()
        );
    }
}

#[test]
fn the_art_table_names_basics() {
    assert_eq!(Art::Time.allowed_basics(), ["Date", "DateTime"]);
    assert_eq!(Art::Records.allowed_basics(), [] as [&str; 0]);
    assert_eq!(Art::Flag.allowed_basics(), ["Bool"]);
    assert_eq!(Art::Number.allowed_basics(), ["Number"]);
    assert!(Art::Text.allowed_basics().contains(&"Choice"));
}

#[test]
fn a_good_text_parses_and_blank_and_comment_lines_are_skipped() {
    let all = parse(&format!("# c\n\n{GOOD}\n# end\n")).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(
        all[0].channels[0].count,
        Count {
            min: 1,
            max: Some(1)
        }
    );
    assert_eq!(parse("").unwrap(), []);
}

#[test]
fn bad_files_are_refused_with_the_line() {
    assert_eq!(
        kind("> items | records | ! | 1\n"),
        ErrorKind::ChannelWithoutForm
    );
    assert_eq!(kind("a 1.0 | A | planned | t\n"), ErrorKind::Shape);
    assert_eq!(kind("a@1.0 | A | planned\n"), ErrorKind::Shape);
    assert_eq!(kind("A@1.0 | A | planned | t\n"), ErrorKind::Name);
    assert_eq!(kind("a@1 | A | planned | t\n"), ErrorKind::Version);
    assert_eq!(kind("a@1.x | A | planned | t\n"), ErrorKind::Version);
    assert_eq!(kind("a@1.0 | A | soon | t\n"), ErrorKind::Status);
    assert_eq!(kind(&format!("{GOOD}{GOOD}")), ErrorKind::DuplicateForm);
    let two = "a@2.0 | A | stable | t\n> items | records | ! | 1\n";
    let both = parse(&format!("{GOOD}{two}")).unwrap();
    assert_eq!(both[1].status, Status::Stable);
    assert_eq!(both[1].version, Version { major: 2, minor: 0 });
    assert_eq!(
        kind(&format!("{GOOD}> items | records | ! | 1\n")),
        ErrorKind::DuplicateChannel
    );
    assert_eq!(
        kind(&format!("{GOOD}> Bad | text | - | 1\n")),
        ErrorKind::ChannelName
    );
    assert_eq!(kind(&format!("{GOOD}> x | blob | - | 1\n")), ErrorKind::Art);
    assert_eq!(
        kind(&format!("{GOOD}> x | text|text | - | 1\n")),
        ErrorKind::ArtDuplicate
    );
    assert_eq!(
        kind(&format!("{GOOD}> x | text|blob | - | 1\n")),
        ErrorKind::Art
    );
    for blank in ["text|", "|text", "text||number"] {
        assert_eq!(
            kind(&format!("{GOOD}> x | {blank} | - | 1\n")),
            ErrorKind::Art,
            "{blank}"
        );
    }
    assert_eq!(kind(&format!("{GOOD}> x |  | - | 1\n")), ErrorKind::Art);
    assert_eq!(
        kind(&format!("{GOOD}> x | text| number | - | 1\n")),
        ErrorKind::Art
    );
    assert_eq!(
        kind(&format!("{GOOD}> x |text|number| - | 1\n")),
        ErrorKind::Shape
    );
    let e = parse(&format!("{GOOD}> x | text|text | - | 1\n")).unwrap_err();
    assert_eq!(e.line, 3);
    for old in ["grouping", "order", "any"] {
        assert_eq!(
            kind(&format!("{GOOD}> x | {old} | - | 1\n")),
            ErrorKind::Art,
            "{old}"
        );
    }
    assert_eq!(
        kind(&format!("{GOOD}> x | text | yes | 1\n")),
        ErrorKind::Required
    );
    assert_eq!(
        kind(&format!("{GOOD}> x | text | - | 3..1\n")),
        ErrorKind::Count
    );
    assert_eq!(
        kind(&format!("{GOOD}> x | text | - | many\n")),
        ErrorKind::Count
    );
    assert_eq!(
        kind(&format!("{GOOD}> x | text | ! | 0..n\n")),
        ErrorKind::RequiredButOptionalCount
    );
    assert_eq!(kind(&format!("{GOOD}> x | text\n")), ErrorKind::Shape);
    assert_eq!(kind("a@1.0 | A | planned | t\n"), ErrorKind::NoChannel);
    assert_eq!(
        kind("a@1.0 | A | planned | t\n> x | text | - | 1\n"),
        ErrorKind::NoRequiredChannel
    );
    let e = parse(&format!("# c\n{GOOD}b@1.0 | B | zzz | t\n")).unwrap_err();
    assert_eq!(e.line, 4);
    assert!(e.to_string().contains("line 4"));
}

#[test]
fn a_version_is_two_plain_numbers() {
    assert_eq!(Version::parse("1.0"), Some(Version { major: 1, minor: 0 }));
    for bad in ["1", "1.", ".1", "1.0.0", "+1.0", "1.-1", " 1.0", "a.b", ""] {
        assert!(Version::parse(bad).is_none(), "{bad:?}");
    }
    assert!(Version { major: 1, minor: 2 } < Version { major: 2, minor: 0 });
}

fn schema_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schema")
}

#[test]
fn form_source_exists_and_the_handshake_is_a_plain_number() {
    use fs_interfaces::formsource_capnp::form_source;
    assert!(std::any::type_name::<form_source::Client>().contains("form_source"));
    assert_eq!(forms::FORM_SOURCE_LEVEL, 1);
    let text = std::fs::read_to_string(schema_dir().join("formsource.capnp")).unwrap();
    assert!(text.contains("@0xfa9420fd361f284b;"));
    assert!(text.contains("hello @0 () -> (level :UInt32"));
    assert!(!text.contains("_v1") && !text.contains("V1 "));
    assert!(
        text.contains("scope :Text"),
        "FormSource.page has the parameter scope"
    );
}

/// The ids of the older contracts did not move (taken from the main head 548be3f).
#[test]
fn the_ids_of_the_older_contracts_are_unchanged() {
    let expected = [
        ("common", "@0xb3f1c2d4e5a69788;"),
        ("layouts", "@0x92216eab3844e15f;"),
        ("bind", "@0xf8c4a79309b02c91;"),
        ("instance", "@0xa36ccf61f34f0d59;"),
    ];
    for (name, id) in expected {
        let text = std::fs::read_to_string(schema_dir().join(format!("{name}.capnp"))).unwrap();
        assert!(text.lines().any(|l| l.trim().starts_with(id)), "{name}");
    }
}
