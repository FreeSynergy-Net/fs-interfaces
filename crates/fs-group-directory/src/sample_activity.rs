//! The life of the sample: rights, invitations of every state, an open edge request, an open mode-change request,
//! one group in the trash.
//!
//! Pattern: Builder (the second half of `sample()`; every step goes through the state operations).

use crate::groups::shared::{Revision, Utc};
use crate::groups::{InvitationAnswer, NestingMode, Right};
use crate::ids::{group_ref, object_id};
use crate::sample::{Book, check};
use crate::state::State;

const DAY_MS: i64 = 24 * 3600 * 1000;
const HOUR_MS: i64 = 3600 * 1000;

/// Rights, invitations (open, accepted, declined, expired), the open requests, the trash.
pub(crate) fn build_activity(s: &mut State, out: &mut Book) {
    let ids = |n: &str| out.id(n).unwrap_or_else(|| object_id(0));
    let fallback = s.asker.clone();
    let who = |n: &str| out.person(n).unwrap_or_else(|| fallback.clone());
    let (assoc, events, outreach, chiefs, trainees) = (
        ids("Association"),
        ids("Events team"),
        ids("Outreach team"),
        ids("Chiefs"),
        ids("Trainees"),
    );
    let alice = who("Alice Chair");
    let mut problems = Vec::new();
    check(
        s.grant(events, &who("Carol Lead"), Right::Manage),
        "grant Carol",
        &mut problems,
    );
    check(
        s.grant(assoc, &who("Bob Deputy"), Right::AddMembers),
        "grant Bob",
        &mut problems,
    );
    check(
        s.grant(assoc, &group_ref(chiefs), Right::RemoveMembers),
        "grant Chiefs",
        &mut problems,
    );
    let at = |ms: i64| Some(Utc::from_millis(s.now.millis() + ms));
    let (later, soon) = (at(30 * DAY_MS), at(HOUR_MS));
    check(
        s.invite(events, &who("Frank Stage"), None, later),
        "invite Frank",
        &mut problems,
    );
    check(
        s.invite(outreach, &who("Hank Union"), None, soon),
        "invite Hank (expires)",
        &mut problems,
    );
    let gina = check(
        s.invite(outreach, &who("Gina Guest"), None, None),
        "invite Gina",
        &mut problems,
    );
    let jonas = check(
        s.invite(events, &who("Jonas Intern"), None, None),
        "invite Jonas",
        &mut problems,
    );
    for (inv, name, answer) in [
        (gina, "Gina Guest", InvitationAnswer::Accept),
        (jonas, "Jonas Intern", InvitationAnswer::Decline),
    ] {
        if let Some(i) = inv {
            s.asker = who(name);
            check(
                s.answer_invitation(i.id, i.revision, answer),
                name,
                &mut problems,
            );
        }
    }
    // Carol manages the Events team only: both requests go to the other side and stay open.
    s.asker = who("Carol Lead");
    check(
        s.request_nesting(outreach, events, NestingMode::Single),
        "request Carol (edge)",
        &mut problems,
    );
    if let Some(e) = s.edge_of(trainees, events).cloned() {
        check(
            s.change_nesting_mode(e.id, e.revision, NestingMode::TopDown),
            "request Carol (mode change)",
            &mut problems,
        );
    }
    s.asker = alice;
    if let Some(archive) = out.id("Archive") {
        let rev = s.alive(archive).map_or(Revision(1), |g| g.revision);
        check(s.delete(archive, rev), "delete Archive", &mut problems);
    }
    // Two hours later: Hank's invitation has run out and reads as expired.
    s.now = Utc::from_millis(s.now.millis() + 2 * HOUR_MS);
    out.problems.extend(problems);
}
