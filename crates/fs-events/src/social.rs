//! Inviting people and answering (dossier A5, A11). Both are a change of the participants plus a typed log entry.
//!
//! Pattern: Facade over `scope::write`; the participant list is built from what the scope sees.

use crate::draft::{
    Answer, Appointment, AppointmentPatch, Attendee, Error, FieldChange, HistoryKind, Invitee,
    ObjectId, Participant, ParticipationStatus, PersonRef, Problem, Result, Revision, Roles,
    SeriesScope, Value, When,
};
use crate::fake::FakeAppointments;
use crate::ids::actor;
use crate::patch::start_value;
use crate::scope;

fn invalid(field: &str, reason: &str) -> Error {
    Error::Invalid(vec![Problem {
        field: field.to_owned(),
        reason: reason.to_owned(),
    }])
}

/// The participants as the scope sees them: an override's list if it has one, else the master's.
fn seen(a: &Appointment, scope: SeriesScope) -> Vec<Participant> {
    if let SeriesScope::This { recurrence_id } = scope {
        let own = a
            .overrides
            .iter()
            .find(|o| o.recurrence_id == recurrence_id)
            .and_then(|o| match &o.change {
                crate::draft::OverrideChange::Changed(p) => p.participants.clone(),
                crate::draft::OverrideChange::Excluded => None,
            });
        if let Some(list) = own {
            return list;
        }
    }
    a.participants.clone()
}

fn participant(
    key: String,
    who: Attendee,
    roles: Roles,
    expect_reply: bool,
    status: ParticipationStatus,
) -> Participant {
    Participant {
        key,
        who,
        roles,
        status,
        expect_reply,
        comment: None,
        send_to: Vec::new(),
        invited_by: None,
        delegated_to: Vec::new(),
        delegated_from: Vec::new(),
        schedule_sequence: 0,
        schedule_updated: None,
    }
}

fn text(s: &str) -> Value {
    Value::Text(s.to_owned())
}

impl FakeAppointments {
    pub(crate) fn invite_impl(
        &self,
        id: ObjectId,
        expected: Revision,
        invitees: &[Invitee],
        scope: SeriesScope,
    ) -> Result<Appointment> {
        let mut s = self.lock();
        let cur = s.live(id)?.appt.clone();
        let mut list = seen(&cur, scope);
        let mut extra = Vec::new();
        if list.is_empty() {
            let me = s.viewer.clone();
            list.push(participant(
                "p1".to_owned(),
                Attendee::Actor(me),
                Roles {
                    owner: true,
                    attendee: true,
                    ..Roles::default()
                },
                false,
                ParticipationStatus::Accepted,
            ));
        }
        for inv in invitees {
            if list.iter().any(|p| p.who == inv.who) {
                continue;
            }
            let key = format!("p{}", list.len() + 1);
            extra.push(FieldChange {
                field: "participant".to_owned(),
                row_key: Some(key.clone()),
                old: None,
                new: Some(Value::Attendee(inv.who.clone())),
            });
            list.push(participant(
                key,
                inv.who.clone(),
                inv.roles,
                inv.expect_reply,
                ParticipationStatus::NeedsAction,
            ));
        }
        let patch = AppointmentPatch {
            participants: Some(list),
            ..AppointmentPatch::default()
        };
        scope::write(
            &mut s,
            id,
            Some(expected),
            &patch,
            scope,
            HistoryKind::Invite,
            extra,
        )
    }

    /// The arguments are those of the trait method `respond` (the trait fixes them).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn respond_impl(
        &self,
        id: ObjectId,
        expected: Revision,
        who: &PersonRef,
        answer: Answer,
        comment: Option<&str>,
        proposal: Option<&When>,
        scope: SeriesScope,
    ) -> Result<Appointment> {
        let mut s = self.lock();
        let cur = scope::checked(&s, id, Some(expected))?;
        let mut list = seen(&cur, scope);
        let Some(p) = list
            .iter_mut()
            .find(|p| actor(p).is_some_and(|x| x.id == who.id))
        else {
            return Err(invalid("who", "is not a participant"));
        };
        let (status, kind) = match answer {
            Answer::Accept => (ParticipationStatus::Accepted, HistoryKind::Accept),
            Answer::Tentative => (ParticipationStatus::Tentative, HistoryKind::TentativeAccept),
            Answer::Decline => (ParticipationStatus::Declined, HistoryKind::Reject),
        };
        let key = p.key.clone();
        let mut extra = vec![FieldChange {
            field: "participant.status".to_owned(),
            row_key: Some(key.clone()),
            old: Some(text(&format!("{:?}", p.status))),
            new: Some(text(&format!("{status:?}"))),
        }];
        p.status = status;
        p.comment = comment.map(str::to_owned);
        if let Some(w) = proposal {
            extra.push(FieldChange {
                field: "proposal.start".to_owned(),
                row_key: Some(key),
                old: None,
                new: Some(start_value(w)),
            });
        }
        let patch = AppointmentPatch {
            participants: Some(list),
            ..AppointmentPatch::default()
        };
        scope::write(&mut s, id, Some(expected), &patch, scope, kind, extra)
    }
}
