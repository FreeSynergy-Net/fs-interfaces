//! The mapping between the locked mode type and the mode of the pure nesting logic.
//!
//! Pattern: Adapter (the two types stay apart so `nesting_logic` can be swapped for `group-nesting`).

use crate::groups::NestingMode;
use crate::nesting_logic::Mode;

/// Locked mode to logic mode.
pub(crate) fn mode_in(mode: NestingMode) -> Mode {
    match mode {
        NestingMode::BottomUp => Mode::BottomUp,
        NestingMode::TopDown => Mode::TopDown,
        NestingMode::Single => Mode::Single,
    }
}
