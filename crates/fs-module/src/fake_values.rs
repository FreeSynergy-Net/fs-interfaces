//! The values of the five standard channels the fake serves for card `c1`.
//!
//! Pattern: Fake Object (fixed example values; `lax` gives wrong ones on purpose).

/// The values of the five standard channels of card `c1`; `lax` breaks each of them on purpose.
pub(crate) fn standard_values(lax: bool) -> Vec<(&'static str, &'static String)> {
    static GOOD: std::sync::LazyLock<[String; 5]> =
        std::sync::LazyLock::new(|| ["A text", "#1e90ff", "PT1H30M", "3", "2"].map(str::to_owned));
    static BAD: std::sync::LazyLock<[String; 5]> =
        std::sync::LazyLock::new(|| ["", "#12", "1h", "0", "10"].map(str::to_owned));
    let v = if lax { &*BAD } else { &*GOOD };
    [
        "description",
        "color",
        "estimated",
        "column-limit",
        "priority",
    ]
    .into_iter()
    .zip(v.iter())
    .collect()
}
