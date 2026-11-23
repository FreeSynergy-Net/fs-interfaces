//! Rules of the operators, part 3: the name and the kind of the group Betreiber are reserved (create AND update).
//!
//! Pattern: Template method (each rule is a mutant killer: let one spelling or one path through and a check fails).

use super::cx::Cx;
use crate::groups::{Error, Field, GroupPatch, ProblemKind};

/// The name of the group of the operators is reserved: no group takes it, in any spelling the fold covers.
pub(super) async fn rules(cx: &mut Cx<'_>, own: &crate::groups::Group) {
    let spellings = [
        "Betreiber",
        "  BETREIBER ",
        "B e t r e i b e r",
        "Betreiber-",
        "\u{FF22}\u{FF45}\u{FF54}\u{FF52}\u{FF45}\u{FF49}\u{FF42}\u{FF45}\u{FF52}",
        "Be\u{200B}treiber",
        "Operators",
    ];
    for name in spellings {
        let mut d = cx.draft("reserved");
        d.name = name.to_owned();
        let made = cx.g.create(d).await;
        cx.ensure(
            reserved_problem(&made, Field::Name),
            &format!("create: the name {name:?} is reserved for the group Betreiber"),
        );
    }
    let mut d = cx.draft("reserved kind");
    d.kind = Some("Betreiber".to_owned());
    cx.ensure(
        reserved_problem(&cx.g.create(d).await, Field::Kind),
        "create: the kind Betreiber is reserved too",
    );
    let patch = GroupPatch {
        name: Some("BETREIBER".to_owned()),
        ..GroupPatch::default()
    };
    cx.ensure(
        reserved_problem(&cx.g.update(own.id, own.revision, patch).await, Field::Name),
        "update: a group is not renamed to the reserved name",
    );
    let kind_patch = GroupPatch {
        kind: Some(Some("Betreiber".to_owned())),
        ..GroupPatch::default()
    };
    cx.ensure(
        reserved_problem(
            &cx.g.update(own.id, own.revision, kind_patch).await,
            Field::Kind,
        ),
        "update: a group does not take the reserved kind",
    );
    let mut d = cx.draft("reserved near");
    "Zzconf Betreiber Hilfe".clone_into(&mut d.name);
    cx.ensure(
        cx.g.create(d).await.is_ok(),
        "create: a name that only contains the word is fine",
    );
}

fn reserved_problem<T>(r: &Result<T, Error>, field: Field) -> bool {
    matches!(r, Err(Error::Invalid(p))
        if p.iter().any(|x| x.field == field && x.problem == ProblemKind::ReservedName))
}
