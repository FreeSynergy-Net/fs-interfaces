//! Input checks of group fields (`Error::Invalid` with the field and the problem).
//!
//! Pattern: Specification (small pure checks; the first problem of each field is reported).

use crate::groups_v1::{Field, FieldProblem, ProblemKind};

/// Longest name.
pub(crate) const NAME_MAX: usize = 200;
/// Longest description.
pub(crate) const TEXT_MAX: usize = 4000;

/// A one-line text: not empty, short, no control characters.
pub(crate) fn text(field: Field, value: &str, max: usize, out: &mut Vec<FieldProblem>) {
    let problem = if value.trim().is_empty() {
        Some(ProblemKind::Empty)
    } else if value.chars().count() > max {
        Some(ProblemKind::TooLong {
            max: u32::try_from(max).unwrap_or(u32::MAX),
        })
    } else if value.chars().any(|c| c.is_control() && c != '\n') {
        Some(ProblemKind::BadCharacters)
    } else {
        None
    };
    out.extend(problem.map(|problem| FieldProblem { field, problem }));
}

/// `mirror` with `sensitive` is refused.
pub(crate) fn mirror(on: bool, sensitive: bool, out: &mut Vec<FieldProblem>) {
    if on && sensitive {
        out.push(FieldProblem {
            field: Field::Mirror,
            problem: ProblemKind::SensitiveNotMirrored,
        });
    }
}

/// There are no AI groups (ADR 0091, 0095): `ai = true` is refused. `groups_v1` keeps the field and has no problem of
/// its own for it, so the one AI problem it has is used (`AiNotMirrored` at `Mirror`: an AI group is never kept).
pub(crate) fn no_ai(ai: bool, out: &mut Vec<FieldProblem>) {
    if ai {
        out.push(FieldProblem {
            field: Field::Mirror,
            problem: ProblemKind::AiNotMirrored,
        });
    }
}
