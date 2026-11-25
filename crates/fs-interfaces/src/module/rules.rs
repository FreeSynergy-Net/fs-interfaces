//! The rules of a data kind: what may be asked and what the answer is.
//!
//! Pattern: Strategy - the rules of a data kind are exchangeable (ADR 0097 E4), the flow in the wrapper stays the same.

use fs_types::Utc;

use super::Context;

/// Pattern: Parameter Object - what is asked of the rules.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Ask<'a> {
    /// Who asks.
    pub who: &'a Context,
    /// The key of the contract (`card.move`), never the host id.
    pub command: &'a str,
    /// The record it is about, if any.
    pub record: Option<&'a str>,
    /// The time for which is asked: "now" (the clock of the module) at a living call, the event's own time at an imported
    /// event. Rules ask "was a member AT `at`", never "is a member now" (user 66/67).
    pub at: Utc,
}

impl<'a> Ask<'a> {
    /// A question for `command` on `record` at `at`.
    #[must_use]
    pub fn new(who: &'a Context, command: &'a str, record: Option<&'a str>, at: Utc) -> Self {
        Self {
            who,
            command,
            record,
            at,
        }
    }
}

/// Pattern: Strategy - the rules of ONE data kind; the module writes them, the wrapper asks them before every change.
/// Deviation: assumes no living caller (`Ask.at`, open `Caller`); there is no method "who is logged in now".
pub trait RightsRules: Send + Sync {
    /// May `ask.who` run `ask.command` on `ask.record` at `ask.at`?
    ///
    /// Decides from `ask.who.person` (and `actor`/`caller`) and `ask.at` only. It NEVER reads `ask.who.claimed`: what a program
    /// says about itself is history, not a right.
    fn may(&self, ask: &Ask<'_>) -> Verdict;

    /// Is the field `field` of `record` hidden from `who`, and why? Required (no default): a default of "nothing hidden"
    /// would be fail-open, so every module must decide, even if the answer is always `None`. Like `may`, it never reads
    /// `who.claimed`.
    fn hidden(&self, who: &Context, record: &str, field: &str) -> Option<HiddenReason>;
}

/// Pattern: Result Object - the answer of the rules. `Deny` becomes `form-source-forbidden` 403.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Verdict {
    /// Allowed.
    Allow,
    /// Not allowed; `reason` is a code, never a foreign sentence.
    Deny {
        /// The code of the reason.
        reason: String,
    },
}

impl Verdict {
    /// `Deny` with a code.
    #[must_use]
    pub fn deny(reason: &str) -> Self {
        Self::Deny {
            reason: reason.into(),
        }
    }

    /// `true` for `Allow`.
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }
}

/// Pattern: Value Object - why a field is hidden (`HiddenReason` of `formsource.capnp`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HiddenReason {
    /// The asking person has no right to the field.
    NoRight,
    /// The field is sensitive.
    Sensitive,
}
