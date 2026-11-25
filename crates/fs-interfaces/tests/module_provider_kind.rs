//! `ProviderKind`: sample data is told by the text of `hello` (`provider == "fake"`), the wire stays text.

use fs_interfaces::module::{ProviderKind, SAMPLE_PROVIDER};

#[test]
fn only_the_sample_word_is_sample_data() {
    assert_eq!(SAMPLE_PROVIDER, "fake");
    assert_eq!(ProviderKind::of("fake"), ProviderKind::Sample);
    for real in [
        "net.freesynergy.Planning",
        "",
        "Fake",
        "fake ",
        "net.freesynergy.fake",
    ] {
        assert_eq!(ProviderKind::of(real), ProviderKind::Module, "{real:?}");
    }
}
