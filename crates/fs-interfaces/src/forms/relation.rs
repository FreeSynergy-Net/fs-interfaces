//! Relations between channels of one form (third kind of line in `forms.txt`: `= interval start end all-day`, `= parent parent`).
//!
//! Pattern: Value Object + Parser - a relation says in data how channels belong together (a closed set of kinds), and the reader checks
//! it; no code of a module or component is needed. Deviation: none.

use super::error::ErrorKind;
use super::types::{Art, Form, Relation};

/// Reads the words after `=`: `interval start end [all-day]` or `parent channel`.
pub(super) fn parse_relation(text: &str) -> Result<Relation, ErrorKind> {
    let words: Vec<&str> = text.split_whitespace().collect();
    match words.as_slice() {
        ["interval", start, end] => Ok(Relation::Interval {
            start: (*start).to_owned(),
            end: (*end).to_owned(),
            all_day: None,
        }),
        ["interval", start, end, all_day] => Ok(Relation::Interval {
            start: (*start).to_owned(),
            end: (*end).to_owned(),
            all_day: Some((*all_day).to_owned()),
        }),
        ["parent", channel] => Ok(Relation::Parent {
            channel: (*channel).to_owned(),
        }),
        ["interval" | "parent", ..] => Err(ErrorKind::RelationShape),
        _ => Err(ErrorKind::RelationKind),
    }
}

/// Every named channel exists, has the art the relation needs, a channel appears once, and a kind is used once per form.
pub(super) fn check_relations(form: &Form) -> Option<(usize, ErrorKind)> {
    let only = |name: &str, art: Art| match form.channel(name) {
        None => Some(ErrorKind::RelationUnknownChannel),
        Some(c) if c.arts != [art] => Some(ErrorKind::RelationArt),
        Some(c) if c.count.max != Some(1) => Some(ErrorKind::RelationArt),
        Some(_) => None,
    };
    for (i, relation) in form.relations.iter().enumerate() {
        if form.relations[..i]
            .iter()
            .any(|o| std::mem::discriminant(o) == std::mem::discriminant(relation))
        {
            return Some((i, ErrorKind::RelationDuplicate));
        }
        let bad = match relation {
            Relation::Interval {
                start,
                end,
                all_day,
            } => {
                let same = start == end || all_day.as_ref().is_some_and(|d| d == start || d == end);
                if same {
                    Some(ErrorKind::RelationDuplicate)
                } else {
                    only(start, Art::Time)
                        .or_else(|| only(end, Art::Time))
                        .or_else(|| all_day.as_deref().and_then(|d| only(d, Art::Flag)))
                }
            }
            Relation::Parent { channel } => only(channel, Art::Reference),
        };
        if let Some(kind) = bad {
            return Some((i, kind));
        }
    }
    None
}
