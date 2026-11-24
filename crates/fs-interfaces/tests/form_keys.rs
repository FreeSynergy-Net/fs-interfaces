//! Form keys: `name@x.y.z` is the one spelling a writer sends, a reader also takes `x.y` (tolerant reader).

use fs_interfaces::forms::{self, Version};
#[test]
fn form_keys_read_both_spellings_and_write_one() {
    let (name, v) = forms::parse_key("cards-in-columns@1.1.0").unwrap();
    assert_eq!((name, &v), ("cards-in-columns", &Version::new(1, 1, 0)));
    assert_eq!(
        forms::parse_key("cards-in-columns@1.1"),
        forms::parse_key("cards-in-columns@1.1.0")
    );
    assert_eq!(forms::key(name, &v), "cards-in-columns@1.1.0");
    for bad in [
        "x",
        "@1.0.0",
        "x@",
        "x@1",
        "x@1.0.0-rc1",
        "x@1.0.0+b",
        "x@1.0.0.0",
        "x@a.b",
        "x@1.-1",
    ] {
        assert!(forms::parse_key(bad).is_none(), "{bad}");
    }
    let form = forms::find_key("cards-in-columns@1.0.0").unwrap();
    assert_eq!(forms::key(&form.name, &form.version), form.key());
}
